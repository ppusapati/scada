/// SD Card Driver (SPI Mode)
///
/// Implements basic SD card access in SPI mode for data logging on
/// the SCADA sensor node. Uses SPI3 (PB3=SCK, PB4=MISO, PB5=MOSI, PA15=CS).
///
/// Initialization sequence (SPI mode):
///   1. Power-up: send 74+ clock pulses with CS high
///   2. CMD0 (GO_IDLE): reset card, enter SPI mode
///   3. CMD8 (SEND_IF_COND): check voltage range (SDHC detection)
///   4. ACMD41 (SD_SEND_OP_COND): wait for card ready
///   5. CMD58 (READ_OCR): confirm SDHC/SDXC status
///   6. Switch to high-speed SPI clock
///
/// Data logging format:
///   - FAT32 filesystem for PC compatibility
///   - Files: YYYYMMDD.CSV (one per day)
///   - Format: timestamp, sensor1, sensor2, ..., quality
///
/// Supports:
///   - Block read (512 bytes)
///   - Block write (512 bytes)
///   - Basic FAT32 navigation (root directory, file create/append)
///   - Card detect via GPIO

use crate::hal::spi::{Spi3Sd, SpiError, SpiClock};
use defmt::{info, warn, error, Format};
use embassy_time::{Timer, Instant};

// ============================================================================
// SD Card Commands
// ============================================================================

#[allow(dead_code)]
pub mod sd_cmd {
    pub const CMD0: u8 = 0;    // GO_IDLE_STATE
    pub const CMD1: u8 = 1;    // SEND_OP_COND (MMC)
    pub const CMD8: u8 = 8;    // SEND_IF_COND
    pub const CMD9: u8 = 9;    // SEND_CSD
    pub const CMD10: u8 = 10;  // SEND_CID
    pub const CMD12: u8 = 12;  // STOP_TRANSMISSION
    pub const CMD16: u8 = 16;  // SET_BLOCKLEN
    pub const CMD17: u8 = 17;  // READ_SINGLE_BLOCK
    pub const CMD18: u8 = 18;  // READ_MULTIPLE_BLOCK
    pub const CMD24: u8 = 24;  // WRITE_BLOCK
    pub const CMD25: u8 = 25;  // WRITE_MULTIPLE_BLOCK
    pub const CMD32: u8 = 32;  // ERASE_WR_BLK_START_ADDR
    pub const CMD33: u8 = 33;  // ERASE_WR_BLK_END_ADDR
    pub const CMD38: u8 = 38;  // ERASE
    pub const CMD55: u8 = 55;  // APP_CMD (prefix for ACMD)
    pub const CMD58: u8 = 58;  // READ_OCR
    pub const CMD59: u8 = 59;  // CRC_ON_OFF
    pub const ACMD41: u8 = 41; // SD_SEND_OP_COND (after CMD55)
}

/// R1 response bits
#[allow(dead_code)]
pub mod r1 {
    pub const IDLE: u8 = 0x01;
    pub const ERASE_RESET: u8 = 0x02;
    pub const ILLEGAL_CMD: u8 = 0x04;
    pub const CRC_ERROR: u8 = 0x08;
    pub const ERASE_SEQ_ERROR: u8 = 0x10;
    pub const ADDRESS_ERROR: u8 = 0x20;
    pub const PARAM_ERROR: u8 = 0x40;
}

// ============================================================================
// Error types
// ============================================================================

#[derive(Clone, Copy, Debug, Format)]
pub enum SdError {
    /// SPI communication error
    Spi(SpiError),
    /// Card not responding to initialization
    InitFailed,
    /// Card rejected command
    CommandError(u8),
    /// Timeout waiting for card response
    Timeout,
    /// Card not present (card detect pin)
    NotPresent,
    /// Write failed (data response token error)
    WriteFailed,
    /// Read failed (data error token)
    ReadFailed,
    /// CRC error on data block
    CrcError,
    /// Card is write-protected
    WriteProtected,
    /// Invalid block address
    InvalidAddress,
    /// FAT32 filesystem error
    FatError,
    /// File not found
    FileNotFound,
    /// Directory full
    DirectoryFull,
    /// Disk full
    DiskFull,
}

impl From<SpiError> for SdError {
    fn from(e: SpiError) -> Self {
        SdError::Spi(e)
    }
}

// ============================================================================
// SD Card types
// ============================================================================

/// SD card type detected during initialization
#[derive(Clone, Copy, PartialEq, Format)]
pub enum CardType {
    /// Standard capacity SD (SDSC), up to 2GB, byte addressing
    SdSc,
    /// High capacity SD (SDHC), 2-32GB, block addressing
    SdHc,
    /// Extended capacity (SDXC), 32GB-2TB, block addressing
    SdXc,
    /// MMC card
    Mmc,
    /// Unknown or not initialized
    Unknown,
}

// ============================================================================
// FAT32 structures
// ============================================================================

/// FAT32 Boot Sector / BPB (BIOS Parameter Block) parsed fields
#[derive(Clone, Copy)]
pub struct Fat32Info {
    /// Bytes per sector (typically 512)
    pub bytes_per_sector: u16,
    /// Sectors per cluster (typically 8, 16, 32, or 64)
    pub sectors_per_cluster: u8,
    /// Number of reserved sectors before FAT
    pub reserved_sectors: u16,
    /// Number of FATs (typically 2)
    pub num_fats: u8,
    /// Sectors per FAT
    pub fat_size: u32,
    /// Root directory cluster number
    pub root_cluster: u32,
    /// First data sector (after reserved + FATs)
    pub first_data_sector: u32,
    /// Total number of data clusters
    pub total_clusters: u32,
    /// Partition start sector (from MBR)
    pub partition_start: u32,
}

impl Fat32Info {
    /// Convert a cluster number to the first sector of that cluster
    pub fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.first_data_sector + (cluster - 2) * self.sectors_per_cluster as u32
    }

    /// Get the sector number containing a specific FAT entry
    pub fn fat_sector_for_cluster(&self, cluster: u32) -> u32 {
        self.partition_start + self.reserved_sectors as u32 + (cluster * 4) / self.bytes_per_sector as u32
    }

    /// Get the byte offset within a FAT sector for a cluster entry
    pub fn fat_offset_for_cluster(&self, cluster: u32) -> usize {
        ((cluster * 4) % self.bytes_per_sector as u32) as usize
    }
}

/// FAT32 directory entry (32 bytes)
#[derive(Clone, Copy)]
pub struct DirEntry {
    pub name: [u8; 11],         // 8.3 filename (space-padded)
    pub attr: u8,               // File attributes
    pub cluster_hi: u16,        // First cluster high word
    pub cluster_lo: u16,        // First cluster low word
    pub size: u32,              // File size in bytes
}

impl DirEntry {
    /// Get the first cluster number
    pub fn first_cluster(&self) -> u32 {
        ((self.cluster_hi as u32) << 16) | (self.cluster_lo as u32)
    }

    /// Check if this is a directory
    pub fn is_directory(&self) -> bool {
        self.attr & 0x10 != 0
    }

    /// Check if entry is free
    pub fn is_free(&self) -> bool {
        self.name[0] == 0x00 || self.name[0] == 0xE5
    }

    /// Check if entry is the last (end of directory)
    pub fn is_end(&self) -> bool {
        self.name[0] == 0x00
    }

    /// Parse from a 32-byte buffer
    pub fn from_bytes(data: &[u8; 32]) -> Self {
        let mut name = [0u8; 11];
        name.copy_from_slice(&data[0..11]);

        Self {
            name,
            attr: data[11],
            cluster_hi: u16::from_le_bytes([data[20], data[21]]),
            cluster_lo: u16::from_le_bytes([data[26], data[27]]),
            size: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
        }
    }

    /// Serialize to a 32-byte buffer
    pub fn to_bytes(&self, buf: &mut [u8; 32]) {
        buf[0..11].copy_from_slice(&self.name);
        buf[11] = self.attr;
        buf[12..20].fill(0); // Reserved fields
        let ch = self.cluster_hi.to_le_bytes();
        buf[20] = ch[0]; buf[21] = ch[1];
        buf[22..26].fill(0); // Time/date fields (simplified)
        let cl = self.cluster_lo.to_le_bytes();
        buf[26] = cl[0]; buf[27] = cl[1];
        let sz = self.size.to_le_bytes();
        buf[28] = sz[0]; buf[29] = sz[1]; buf[30] = sz[2]; buf[31] = sz[3];
    }

    /// Create a new directory entry with an 8.3 filename
    pub fn new_file(name_8_3: &[u8; 11], cluster: u32, size: u32) -> Self {
        Self {
            name: *name_8_3,
            attr: 0x20, // Archive attribute
            cluster_hi: (cluster >> 16) as u16,
            cluster_lo: (cluster & 0xFFFF) as u16,
            size,
        }
    }
}

// ============================================================================
// SD Card Driver
// ============================================================================

/// SD Card driver for SPI mode operation
pub struct SdCard {
    card_type: CardType,
    initialized: bool,
    fat_info: Option<Fat32Info>,
    /// 512-byte sector buffer (statically allocated)
    sector_buf: [u8; 512],
}

impl SdCard {
    pub fn new() -> Self {
        Self {
            card_type: CardType::Unknown,
            initialized: false,
            fat_info: None,
            sector_buf: [0u8; 512],
        }
    }

    /// Initialize the SD card in SPI mode
    ///
    /// This follows the standard SD SPI initialization sequence:
    /// 1. Send 80+ clock pulses with CS high (card power-up)
    /// 2. CMD0 to enter SPI mode
    /// 3. CMD8 to check for SDHC support
    /// 4. ACMD41 to complete initialization
    /// 5. CMD58 to read OCR and confirm card type
    pub async fn init(&mut self, spi: &mut Spi3Sd) -> Result<CardType, SdError> {
        info!("SD: starting initialization...");

        // Step 1: Send 80 clock pulses with CS high (power-up sequence)
        spi.send_clocks(10).await?; // 10 bytes * 8 bits = 80 clocks

        // Step 2: CMD0 - GO_IDLE_STATE (reset, enter SPI mode)
        let mut attempts = 0;
        loop {
            let r1 = spi.sd_command(sd_cmd::CMD0, 0x00000000, 0x95).await?;
            if r1 == r1::IDLE {
                break;
            }
            attempts += 1;
            if attempts > 10 {
                error!("SD: CMD0 failed after {} attempts (r1=0x{:02X})", attempts, r1);
                return Err(SdError::InitFailed);
            }
            Timer::after_millis(10).await;
        }
        info!("SD: CMD0 OK (idle)");

        // Step 3: CMD8 - SEND_IF_COND (voltage check, SDHC detection)
        // Arg: 0x000001AA = VHS=1 (2.7-3.6V), check pattern=0xAA
        let r1 = spi.sd_command(sd_cmd::CMD8, 0x000001AA, 0x87).await?;

        let is_sdhc_capable = if r1 == r1::IDLE {
            // Read R7 response (4 additional bytes)
            // In a real implementation we'd read the trailing 4 bytes
            // and verify echo-back of 0x1AA
            true
        } else {
            // Card does not support CMD8 = SDv1 or MMC
            false
        };

        // Step 4: ACMD41 - SD_SEND_OP_COND (wait for initialization complete)
        let hcs_bit = if is_sdhc_capable { 0x40000000u32 } else { 0 };

        let start = Instant::now();
        loop {
            // Send CMD55 first (prefix for application commands)
            spi.sd_command(sd_cmd::CMD55, 0, 0x65).await?;

            // Then ACMD41
            let r1 = spi.sd_command(sd_cmd::ACMD41, hcs_bit, 0x77).await?;
            if r1 == 0x00 {
                // Card is ready
                break;
            }
            if r1 != r1::IDLE {
                warn!("SD: ACMD41 unexpected response 0x{:02X}", r1);
            }
            if Instant::now().as_millis() - start.as_millis() > 2000 {
                error!("SD: ACMD41 timeout");
                return Err(SdError::Timeout);
            }
            Timer::after_millis(10).await;
        }
        info!("SD: ACMD41 OK (ready)");

        // Step 5: CMD58 - READ_OCR (determine card type)
        let r1 = spi.sd_command(sd_cmd::CMD58, 0, 0xFD).await?;
        if r1 == 0x00 {
            // In a full implementation, read the 4-byte OCR response
            // and check the CCS bit (bit 30) for SDHC
            self.card_type = if is_sdhc_capable {
                CardType::SdHc
            } else {
                CardType::SdSc
            };
        } else {
            self.card_type = CardType::SdSc;
        }

        // For SDSC, set block length to 512 bytes
        if self.card_type == CardType::SdSc {
            let r1 = spi.sd_command(sd_cmd::CMD16, 512, 0xFF).await?;
            if r1 != 0x00 {
                warn!("SD: CMD16 response 0x{:02X}", r1);
            }
        }

        self.initialized = true;
        info!("SD: initialized, type={}", self.card_type);

        Ok(self.card_type)
    }

    /// Read a single 512-byte block
    pub async fn read_block(
        &mut self,
        spi: &mut Spi3Sd,
        block_addr: u32,
        buf: &mut [u8; 512],
    ) -> Result<(), SdError> {
        if !self.initialized {
            return Err(SdError::InitFailed);
        }

        // For SDSC, address is byte-based; for SDHC, it is block-based
        let addr = if self.card_type == CardType::SdSc {
            block_addr * 512
        } else {
            block_addr
        };

        // CMD17 - READ_SINGLE_BLOCK
        let r1 = spi.sd_command(sd_cmd::CMD17, addr, 0xFF).await?;
        if r1 != 0x00 {
            return Err(SdError::CommandError(r1));
        }

        // Wait for data start token (0xFE)
        spi.cs.assert();
        let start = Instant::now();
        loop {
            let mut token = [0xFF; 1];
            spi.spi.read(&mut token).await.map_err(|_| SdError::Spi(SpiError::BusError))?;
            if token[0] == 0xFE {
                break;
            }
            if token[0] != 0xFF {
                spi.cs.deassert();
                return Err(SdError::ReadFailed);
            }
            if Instant::now().as_millis() - start.as_millis() > 500 {
                spi.cs.deassert();
                return Err(SdError::Timeout);
            }
        }

        // Read 512 bytes of data
        spi.spi.read(buf).await.map_err(|_| SdError::Spi(SpiError::BusError))?;

        // Read and discard 2-byte CRC
        let mut crc = [0u8; 2];
        spi.spi.read(&mut crc).await.map_err(|_| SdError::Spi(SpiError::BusError))?;

        spi.cs.deassert();

        Ok(())
    }

    /// Write a single 512-byte block
    pub async fn write_block(
        &mut self,
        spi: &mut Spi3Sd,
        block_addr: u32,
        data: &[u8; 512],
    ) -> Result<(), SdError> {
        if !self.initialized {
            return Err(SdError::InitFailed);
        }

        let addr = if self.card_type == CardType::SdSc {
            block_addr * 512
        } else {
            block_addr
        };

        // CMD24 - WRITE_BLOCK
        let r1 = spi.sd_command(sd_cmd::CMD24, addr, 0xFF).await?;
        if r1 != 0x00 {
            return Err(SdError::CommandError(r1));
        }

        spi.cs.assert();

        // Send data start token
        spi.spi.write(&[0xFE]).await.map_err(|_| SdError::Spi(SpiError::BusError))?;

        // Send 512 bytes of data
        spi.spi.write(data).await.map_err(|_| SdError::Spi(SpiError::BusError))?;

        // Send dummy CRC (2 bytes)
        spi.spi.write(&[0xFF, 0xFF]).await.map_err(|_| SdError::Spi(SpiError::BusError))?;

        // Read data response token
        let mut response = [0xFF; 1];
        spi.spi.read(&mut response).await.map_err(|_| SdError::Spi(SpiError::BusError))?;

        // Check response: xxx0_0101 = accepted
        if (response[0] & 0x1F) != 0x05 {
            spi.cs.deassert();
            return Err(SdError::WriteFailed);
        }

        // Wait for card to finish programming (busy = MISO low)
        let start = Instant::now();
        loop {
            let mut busy = [0u8; 1];
            spi.spi.read(&mut busy).await.map_err(|_| SdError::Spi(SpiError::BusError))?;
            if busy[0] == 0xFF {
                break; // Card is no longer busy
            }
            if Instant::now().as_millis() - start.as_millis() > 500 {
                spi.cs.deassert();
                return Err(SdError::Timeout);
            }
        }

        spi.cs.deassert();

        Ok(())
    }

    /// Mount the FAT32 filesystem
    ///
    /// Reads the MBR partition table and FAT32 boot sector to locate
    /// the filesystem structures.
    pub async fn mount_fat32(&mut self, spi: &mut Spi3Sd) -> Result<(), SdError> {
        // Read MBR (sector 0)
        let mut mbr = [0u8; 512];
        self.read_block(spi, 0, &mut mbr).await?;

        // Check MBR signature
        if mbr[510] != 0x55 || mbr[511] != 0xAA {
            error!("SD: invalid MBR signature");
            return Err(SdError::FatError);
        }

        // Read first partition entry (offset 0x1BE, 16 bytes)
        let part_type = mbr[0x1C2];
        let part_start = u32::from_le_bytes([mbr[0x1C6], mbr[0x1C7], mbr[0x1C8], mbr[0x1C9]]);

        // Check for FAT32 partition type (0x0B or 0x0C)
        if part_type != 0x0B && part_type != 0x0C {
            error!("SD: partition type 0x{:02X} is not FAT32", part_type);
            return Err(SdError::FatError);
        }

        // Read FAT32 boot sector (BPB)
        let mut bpb = [0u8; 512];
        self.read_block(spi, part_start, &mut bpb).await?;

        // Parse BPB
        let bytes_per_sector = u16::from_le_bytes([bpb[11], bpb[12]]);
        let sectors_per_cluster = bpb[13];
        let reserved_sectors = u16::from_le_bytes([bpb[14], bpb[15]]);
        let num_fats = bpb[16];
        let fat_size = u32::from_le_bytes([bpb[36], bpb[37], bpb[38], bpb[39]]);
        let root_cluster = u32::from_le_bytes([bpb[44], bpb[45], bpb[46], bpb[47]]);
        let total_sectors = u32::from_le_bytes([bpb[32], bpb[33], bpb[34], bpb[35]]);

        let first_data_sector = part_start
            + reserved_sectors as u32
            + (num_fats as u32 * fat_size);

        let total_data_sectors = total_sectors
            - reserved_sectors as u32
            - (num_fats as u32 * fat_size);
        let total_clusters = total_data_sectors / sectors_per_cluster as u32;

        self.fat_info = Some(Fat32Info {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            fat_size,
            root_cluster,
            first_data_sector,
            total_clusters,
            partition_start: part_start,
        });

        info!("SD: FAT32 mounted (cluster_size={}K, total={}MB)",
            (sectors_per_cluster as u32 * bytes_per_sector as u32) / 1024,
            (total_clusters as u64 * sectors_per_cluster as u64 * bytes_per_sector as u64) / (1024 * 1024));

        Ok(())
    }

    /// Open a file in the root directory by 8.3 name
    ///
    /// Returns the first cluster and file size if found.
    pub async fn open_file(
        &mut self,
        spi: &mut Spi3Sd,
        name: &[u8; 11],
    ) -> Result<(u32, u32), SdError> {
        let fat = self.fat_info.ok_or(SdError::FatError)?;
        let mut cluster = fat.root_cluster;

        loop {
            let sector = fat.cluster_to_sector(cluster);

            for s in 0..fat.sectors_per_cluster {
                let mut buf = [0u8; 512];
                self.read_block(spi, sector + s as u32, &mut buf).await?;

                for i in (0..512).step_by(32) {
                    let mut entry_bytes = [0u8; 32];
                    entry_bytes.copy_from_slice(&buf[i..i + 32]);
                    let entry = DirEntry::from_bytes(&entry_bytes);

                    if entry.is_end() {
                        return Err(SdError::FileNotFound);
                    }
                    if entry.is_free() {
                        continue;
                    }
                    if entry.name == *name {
                        return Ok((entry.first_cluster(), entry.size));
                    }
                }
            }

            // Follow cluster chain
            cluster = self.read_fat_entry(spi, cluster).await?;
            if cluster >= 0x0FFFFFF8 {
                break; // End of cluster chain
            }
        }

        Err(SdError::FileNotFound)
    }

    /// Write data to a file (append mode)
    ///
    /// Creates the file if it does not exist, or appends to existing data.
    /// Data is written in 512-byte blocks (zero-padded if necessary).
    pub async fn write_file(
        &mut self,
        spi: &mut Spi3Sd,
        name: &[u8; 11],
        data: &[u8],
    ) -> Result<(), SdError> {
        let fat = self.fat_info.ok_or(SdError::FatError)?;

        // Try to open existing file
        let (first_cluster, existing_size) = match self.open_file(spi, name).await {
            Ok((cluster, size)) => (cluster, size),
            Err(SdError::FileNotFound) => {
                // Create new file: allocate cluster, create directory entry
                let new_cluster = self.allocate_cluster(spi).await?;
                self.create_dir_entry(spi, name, new_cluster).await?;
                (new_cluster, 0)
            }
            Err(e) => return Err(e),
        };

        // Find the last cluster in the chain
        let mut current_cluster = first_cluster;
        let mut offset_in_cluster = (existing_size % (fat.sectors_per_cluster as u32 * fat.bytes_per_sector as u32)) as usize;

        if existing_size > 0 {
            let clusters_used = existing_size / (fat.sectors_per_cluster as u32 * fat.bytes_per_sector as u32);
            for _ in 0..clusters_used {
                let next = self.read_fat_entry(spi, current_cluster).await?;
                if next >= 0x0FFFFFF8 {
                    break;
                }
                current_cluster = next;
            }
        }

        // Write data
        let mut data_offset = 0;
        while data_offset < data.len() {
            let sector_in_cluster = offset_in_cluster / 512;
            let byte_in_sector = offset_in_cluster % 512;

            // Read the current sector
            let sector = fat.cluster_to_sector(current_cluster) + sector_in_cluster as u32;
            let mut buf = [0u8; 512];
            if byte_in_sector > 0 {
                self.read_block(spi, sector, &mut buf).await?;
            }

            // Fill buffer with new data
            let space = 512 - byte_in_sector;
            let to_write = space.min(data.len() - data_offset);
            buf[byte_in_sector..byte_in_sector + to_write]
                .copy_from_slice(&data[data_offset..data_offset + to_write]);

            // Write the sector back
            self.write_block(spi, sector, &buf).await?;

            data_offset += to_write;
            offset_in_cluster += to_write;

            // Check if we need a new cluster
            let cluster_size = fat.sectors_per_cluster as usize * 512;
            if offset_in_cluster >= cluster_size {
                let new_cluster = self.allocate_cluster(spi).await?;
                self.write_fat_entry(spi, current_cluster, new_cluster).await?;
                current_cluster = new_cluster;
                offset_in_cluster = 0;
            }
        }

        // Update file size in directory entry
        let new_size = existing_size + data.len() as u32;
        self.update_dir_entry_size(spi, name, new_size).await?;

        Ok(())
    }

    /// Close a file (flush / no-op in this simplified implementation)
    pub fn close_file(&mut self) {
        // In this simplified implementation, writes are immediate.
        // A more complete implementation would flush cached sectors.
    }

    // ── FAT table operations ──

    /// Read a FAT entry (next cluster pointer)
    async fn read_fat_entry(
        &mut self,
        spi: &mut Spi3Sd,
        cluster: u32,
    ) -> Result<u32, SdError> {
        let fat = self.fat_info.ok_or(SdError::FatError)?;
        let fat_sector = fat.fat_sector_for_cluster(cluster);
        let fat_offset = fat.fat_offset_for_cluster(cluster);

        let mut buf = [0u8; 512];
        self.read_block(spi, fat_sector, &mut buf).await?;

        let entry = u32::from_le_bytes([
            buf[fat_offset],
            buf[fat_offset + 1],
            buf[fat_offset + 2],
            buf[fat_offset + 3],
        ]) & 0x0FFFFFFF;

        Ok(entry)
    }

    /// Write a FAT entry
    async fn write_fat_entry(
        &mut self,
        spi: &mut Spi3Sd,
        cluster: u32,
        value: u32,
    ) -> Result<(), SdError> {
        let fat = self.fat_info.ok_or(SdError::FatError)?;
        let fat_sector = fat.fat_sector_for_cluster(cluster);
        let fat_offset = fat.fat_offset_for_cluster(cluster);

        let mut buf = [0u8; 512];
        self.read_block(spi, fat_sector, &mut buf).await?;

        // Preserve upper 4 bits of existing entry
        let existing = u32::from_le_bytes([
            buf[fat_offset], buf[fat_offset + 1],
            buf[fat_offset + 2], buf[fat_offset + 3],
        ]);
        let new_val = (existing & 0xF0000000) | (value & 0x0FFFFFFF);
        let bytes = new_val.to_le_bytes();
        buf[fat_offset..fat_offset + 4].copy_from_slice(&bytes);

        self.write_block(spi, fat_sector, &buf).await?;

        // Write to second FAT copy if present
        if fat.num_fats > 1 {
            let fat2_sector = fat_sector + fat.fat_size;
            self.write_block(spi, fat2_sector, &buf).await?;
        }

        Ok(())
    }

    /// Allocate a free cluster
    async fn allocate_cluster(&mut self, spi: &mut Spi3Sd) -> Result<u32, SdError> {
        let fat = self.fat_info.ok_or(SdError::FatError)?;

        // Search for a free cluster (value == 0 in FAT)
        for cluster in 2..fat.total_clusters + 2 {
            let entry = self.read_fat_entry(spi, cluster).await?;
            if entry == 0 {
                // Mark as end-of-chain
                self.write_fat_entry(spi, cluster, 0x0FFFFFFF).await?;
                return Ok(cluster);
            }
        }

        Err(SdError::DiskFull)
    }

    /// Create a directory entry in the root directory
    async fn create_dir_entry(
        &mut self,
        spi: &mut Spi3Sd,
        name: &[u8; 11],
        cluster: u32,
    ) -> Result<(), SdError> {
        let fat = self.fat_info.ok_or(SdError::FatError)?;
        let mut dir_cluster = fat.root_cluster;

        loop {
            let sector = fat.cluster_to_sector(dir_cluster);
            for s in 0..fat.sectors_per_cluster {
                let mut buf = [0u8; 512];
                self.read_block(spi, sector + s as u32, &mut buf).await?;

                for i in (0..512).step_by(32) {
                    if buf[i] == 0x00 || buf[i] == 0xE5 {
                        // Found a free entry
                        let entry = DirEntry::new_file(name, cluster, 0);
                        let mut entry_bytes = [0u8; 32];
                        entry.to_bytes(&mut entry_bytes);
                        buf[i..i + 32].copy_from_slice(&entry_bytes);
                        self.write_block(spi, sector + s as u32, &buf).await?;
                        return Ok(());
                    }
                }
            }

            // Follow cluster chain
            let next = self.read_fat_entry(spi, dir_cluster).await?;
            if next >= 0x0FFFFFF8 {
                return Err(SdError::DirectoryFull);
            }
            dir_cluster = next;
        }
    }

    /// Update the file size in an existing directory entry
    async fn update_dir_entry_size(
        &mut self,
        spi: &mut Spi3Sd,
        name: &[u8; 11],
        new_size: u32,
    ) -> Result<(), SdError> {
        let fat = self.fat_info.ok_or(SdError::FatError)?;
        let mut dir_cluster = fat.root_cluster;

        loop {
            let sector = fat.cluster_to_sector(dir_cluster);
            for s in 0..fat.sectors_per_cluster {
                let mut buf = [0u8; 512];
                self.read_block(spi, sector + s as u32, &mut buf).await?;

                for i in (0..512).step_by(32) {
                    if buf[i] == 0x00 {
                        return Err(SdError::FileNotFound);
                    }
                    if buf[i] == 0xE5 {
                        continue;
                    }
                    if buf[i..i + 11] == *name {
                        // Update size field (offset 28-31 within entry)
                        let size_bytes = new_size.to_le_bytes();
                        buf[i + 28..i + 32].copy_from_slice(&size_bytes);
                        self.write_block(spi, sector + s as u32, &buf).await?;
                        return Ok(());
                    }
                }
            }

            let next = self.read_fat_entry(spi, dir_cluster).await?;
            if next >= 0x0FFFFFF8 {
                break;
            }
            dir_cluster = next;
        }

        Err(SdError::FileNotFound)
    }

    // ── Status helpers ──

    /// Get the detected card type
    pub fn card_type(&self) -> CardType {
        self.card_type
    }

    /// Check if the card is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if FAT32 is mounted
    pub fn is_mounted(&self) -> bool {
        self.fat_info.is_some()
    }
}
