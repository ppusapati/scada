/* STM32MP157 Cortex-M4 Memory Layout
 *
 * The M4 co-processor on STM32MP157 has its own dedicated memories:
 *
 *   RETRAM  (64 KB)  @ 0x00000000 - Retained RAM, survives low-power modes
 *   MCUSRAM (256 KB)  @ 0x10000000 - Main M4 SRAM, split into multiple banks
 *
 * Memory partitioning for this firmware:
 *   - Code + read-only data:     RETRAM  (first 48 KB)
 *   - Stack + BSS + heap:        MCUSRAM1 (128 KB)
 *   - RPMsg shared memory:       MCUSRAM2 (128 KB) — shared with A7 via OpenAMP
 *   - Resource table:            RETRAM end (last 16 KB)
 *
 * The resource table is placed at a known address so the A7 Linux
 * remoteproc driver can discover RPMsg virtio rings.
 *
 * IMPORTANT: MCUSRAM2 (0x10020000-0x1003FFFF) is reserved for OpenAMP
 * shared buffers. The A7 Linux kernel maps this region via the device tree.
 * M4 firmware must NOT use this region for general-purpose data.
 */

MEMORY
{
    /* Retained RAM — code and critical data (survives standby) */
    RETRAM (rwx)     : ORIGIN = 0x00000000, LENGTH = 48K

    /* Resource table — placed at fixed address for remoteproc discovery */
    RESTBL (r)       : ORIGIN = 0x0000C000, LENGTH = 16K

    /* M4 SRAM Bank 1 — stack, BSS, heap, general data */
    MCUSRAM1 (rwx)   : ORIGIN = 0x10000000, LENGTH = 128K

    /* M4 SRAM Bank 2 — OpenAMP/RPMsg shared memory (DO NOT USE FOR GENERAL DATA) */
    MCUSRAM2 (rw)    : ORIGIN = 0x10020000, LENGTH = 128K
}

/* Stack configuration */
_stack_start = ORIGIN(MCUSRAM1) + LENGTH(MCUSRAM1);

/* OpenAMP shared memory region addresses */
_shmem_base     = ORIGIN(MCUSRAM2);
_shmem_size     = LENGTH(MCUSRAM2);

/* VirtIO ring buffer addresses within shared memory
 * Layout:
 *   0x10020000 - 0x10027FFF : VirtIO Ring 0 (M4 → A7, TX) — 32 KB
 *   0x10028000 - 0x1002FFFF : VirtIO Ring 1 (A7 → M4, RX) — 32 KB
 *   0x10030000 - 0x1003FFFF : RPMsg buffer pool — 64 KB
 */
_vring0_base    = 0x10020000;
_vring0_size    = 0x00008000;
_vring1_base    = 0x10028000;
_vring1_size    = 0x00008000;
_rpmsg_buf_base = 0x10030000;
_rpmsg_buf_size = 0x00010000;

/* Resource table placed in RESTBL section */
SECTIONS
{
    .resource_table :
    {
        . = ALIGN(4);
        KEEP(*(.resource_table))
        . = ALIGN(4);
    } > RESTBL
}
