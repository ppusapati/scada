/* STM32H743VIT6 Memory Layout */
/* Flash: 2MB, SRAM: 1MB total (multiple regions) */

MEMORY
{
    /* FLASH - 2MB, dual bank */
    FLASH  (rx)  : ORIGIN = 0x08000000, LENGTH = 2048K

    /* DTCM RAM - 128KB, zero wait state, tightly coupled */
    /* Used for stack and time-critical data */
    DTCM   (rwx) : ORIGIN = 0x20000000, LENGTH = 128K

    /* AXI SRAM (D1) - 512KB */
    /* Main RAM for heap, DMA buffers, large data structures */
    RAM    (rwx) : ORIGIN = 0x24000000, LENGTH = 512K

    /* SRAM1 (D2) - 128KB */
    /* Used for DMA-accessible peripheral buffers */
    SRAM1  (rwx) : ORIGIN = 0x30000000, LENGTH = 128K

    /* SRAM2 (D2) - 128KB */
    /* Used for Ethernet/USB DMA buffers */
    SRAM2  (rwx) : ORIGIN = 0x30020000, LENGTH = 128K

    /* SRAM3 (D2) - 32KB */
    SRAM3  (rwx) : ORIGIN = 0x30040000, LENGTH = 32K

    /* SRAM4 (D3) - 64KB, accessible in low-power modes */
    /* Used for backup/persistent data */
    SRAM4  (rwx) : ORIGIN = 0x38000000, LENGTH = 64K

    /* Backup SRAM - 4KB, battery-backed */
    BKPSRAM (rw) : ORIGIN = 0x38800000, LENGTH = 4K
}

/* Stack in DTCM for zero wait state access */
_stack_start = ORIGIN(DTCM) + LENGTH(DTCM);

/* Embassy executor uses main RAM */
SECTIONS
{
    /* DMA buffers must be in D2 SRAM for peripheral access */
    .dma_buffers (NOLOAD) :
    {
        *(.dma_buffers)
        *(.dma_buffers.*)
    } > SRAM1

    /* Ethernet DMA buffers */
    .eth_buffers (NOLOAD) :
    {
        *(.eth_buffers)
        *(.eth_buffers.*)
    } > SRAM2
}
