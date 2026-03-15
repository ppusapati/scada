//! Over-The-Air (OTA) Firmware Update Module
//!
//! Provides secure firmware updates after programming interfaces are locked.
//! Uses STM32H743 dual-bank flash for safe, atomic updates with rollback.
//!
//! Update flow:
//! 1. Server sends firmware metadata (version, size, CRC, signature)
//! 2. Device validates: newer version, compatible hardware, sufficient space
//! 3. Firmware chunks downloaded via any comm channel (ETH/WiFi/GSM/LoRa/BLE)
//! 4. Chunks written to Bank 2 (OTA staging area)
//! 5. Full image CRC-32 verified after download complete
//! 6. HMAC-SHA256 signature verified (if signed build)
//! 7. Bank swap via option byte + system reset
//! 8. New firmware boots and confirms success
//! 9. If boot fails, watchdog triggers rollback to previous bank

pub mod firmware_update;
