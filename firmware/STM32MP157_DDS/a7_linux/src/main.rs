/// STM32MP157 Cortex-A7 DDS Application
///
/// P9e Microsystems Pvt Ltd
///
/// This application runs on the A7 Linux core of the STM32MP157
/// dual-core processor. It acts as a bridge between the M4 real-time
/// co-processor and the DDS network:
///
///   M4 (sensors) ──RPMsg──→ A7 (this app) ──DDS──→ Network
///   M4 (actuators) ←──RPMsg── A7 ←──DDS── Network
///
/// Architecture:
///   ┌──────────────────────────────────────────────────────────────┐
///   │ A7 Linux Application                                         │
///   ├─────────────┬──────────────┬───────────┬───────────┬────────┤
///   │ RPMsg       │ DDS          │ Modbus    │ PTP       │ OTA    │
///   │ /dev/ttyR.. │ Participant  │ TCP GW    │ ptp4l     │ HTTP   │
///   │ XRCE Agent  │ Pub/Sub      │ Port 502  │ phc2sys   │ Update │
///   └─────────────┴──────────────┴───────────┴───────────┴────────┘
///
/// Startup sequence:
///   1. Load configuration from /etc/scada/config.toml
///   2. Initialize DDS participant
///   3. Open RPMsg device (/dev/ttyRPMSG0)
///   4. Start XRCE-DDS Agent (M4 ↔ DDS bridge)
///   5. Start Modbus TCP gateway (DDS ↔ Modbus bridge)
///   6. Start PTP synchronization monitoring
///   7. Start OTA update listener
///   8. Enter main event loop
///
/// Signal handling:
///   SIGTERM/SIGINT → graceful shutdown (flush DDS, close RPMsg)

mod dds;
mod rpmsg;
mod modbus;
mod ptp;
mod config;
mod ota;

use anyhow::Result;
use clap::Parser;
use tracing::{info, warn, error};
use tokio::signal::unix::{signal, SignalKind};
use tokio_util::sync::CancellationToken;

/// SCADA DDS Gateway — STM32MP157 A7 Application
#[derive(Parser, Debug)]
#[command(name = "scada-dds", version = "1.0.0")]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "/etc/scada/config.toml")]
    config: String,

    /// DDS domain ID (overrides config file)
    #[arg(short, long)]
    domain: Option<u16>,

    /// RPMsg device path
    #[arg(short, long, default_value = "/dev/ttyRPMSG0")]
    rpmsg_device: String,

    /// Enable Modbus TCP gateway
    #[arg(long, default_value = "true")]
    modbus_gateway: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&args.log_level))
        )
        .with_target(true)
        .with_thread_ids(true)
        .init();

    info!("═══════════════════════════════════════════════════");
    info!("  SCADA DDS Gateway — STM32MP157 A7 Application");
    info!("  P9e Microsystems Pvt Ltd");
    info!("  Version 1.0.0");
    info!("═══════════════════════════════════════════════════");

    // Load configuration
    let cfg = config::ScadaConfig::load(&args.config)?;
    info!("Configuration loaded from {}", args.config);

    let domain_id = args.domain.unwrap_or(cfg.dds.domain_id);
    info!("DDS Domain ID: {}", domain_id);

    // Cancellation token for graceful shutdown
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    // Signal handler for graceful shutdown
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt())
            .expect("Failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown...");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, initiating graceful shutdown...");
            }
        }

        cancel_clone.cancel();
    });

    // ── Initialize DDS participant ──
    let (dds_publisher, dds_subscriber) = dds::participant::create_participant(
        domain_id,
        &cfg,
    )?;
    info!("DDS participant created (domain={})", domain_id);

    // ── Open RPMsg device ──
    let rpmsg_handle = rpmsg::RpmsgHandle::open(&args.rpmsg_device)?;
    info!("RPMsg device opened: {}", args.rpmsg_device);

    // ── Start XRCE-DDS Agent (RPMsg ↔ DDS bridge) ──
    let rpmsg_cancel = cancel.clone();
    let rpmsg_publisher = dds_publisher.clone();
    let rpmsg_cfg = cfg.clone();
    let rpmsg_task = tokio::spawn(async move {
        info!("Starting XRCE-DDS Agent...");
        if let Err(e) = rpmsg::run_xrce_agent(
            rpmsg_handle,
            rpmsg_publisher,
            &rpmsg_cfg,
            rpmsg_cancel,
        ).await {
            error!("XRCE-DDS Agent error: {}", e);
        }
    });

    // ── Start DDS publisher (forwards M4 data to network) ──
    let pub_cancel = cancel.clone();
    let publisher_cfg = cfg.clone();
    let pub_task = tokio::spawn(async move {
        info!("Starting DDS publishers...");
        if let Err(e) = dds::publisher::run_publishers(
            dds_publisher,
            &publisher_cfg,
            pub_cancel,
        ).await {
            error!("DDS publisher error: {}", e);
        }
    });

    // ── Start DDS subscriber (receives commands from network) ──
    let sub_cancel = cancel.clone();
    let subscriber_cfg = cfg.clone();
    let sub_task = tokio::spawn(async move {
        info!("Starting DDS subscribers...");
        if let Err(e) = dds::subscriber::run_subscribers(
            dds_subscriber,
            &subscriber_cfg,
            sub_cancel,
        ).await {
            error!("DDS subscriber error: {}", e);
        }
    });

    // ── Start Modbus TCP gateway (optional) ──
    let modbus_task = if args.modbus_gateway {
        let modbus_cancel = cancel.clone();
        let modbus_cfg = cfg.clone();
        Some(tokio::spawn(async move {
            info!("Starting Modbus TCP gateway on port {}...", modbus_cfg.modbus.tcp_port);
            if let Err(e) = modbus::gateway::run_modbus_gateway(
                &modbus_cfg,
                modbus_cancel,
            ).await {
                error!("Modbus gateway error: {}", e);
            }
        }))
    } else {
        None
    };

    // ── Start PTP synchronization monitor ──
    let ptp_cancel = cancel.clone();
    let ptp_cfg = cfg.clone();
    let ptp_task = tokio::spawn(async move {
        info!("Starting PTP synchronization monitor...");
        if let Err(e) = ptp::run_ptp_monitor(&ptp_cfg, ptp_cancel).await {
            error!("PTP monitor error: {}", e);
        }
    });

    // ── Start OTA update listener ──
    let ota_cancel = cancel.clone();
    let ota_cfg = cfg.clone();
    let ota_task = tokio::spawn(async move {
        info!("Starting OTA update listener...");
        if let Err(e) = ota::run_ota_listener(&ota_cfg, ota_cancel).await {
            error!("OTA listener error: {}", e);
        }
    });

    info!("All services started. SCADA DDS Gateway running.");

    // Wait for cancellation
    cancel.cancelled().await;

    info!("Shutting down...");

    // Wait for all tasks to complete
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        async {
            let _ = rpmsg_task.await;
            let _ = pub_task.await;
            let _ = sub_task.await;
            if let Some(mt) = modbus_task {
                let _ = mt.await;
            }
            let _ = ptp_task.await;
            let _ = ota_task.await;
        }
    ).await;

    info!("SCADA DDS Gateway shutdown complete.");
    Ok(())
}
