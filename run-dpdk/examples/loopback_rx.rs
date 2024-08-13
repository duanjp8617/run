use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};

use arrayvec::ArrayVec;
use ctrlc;
use once_cell::sync::OnceCell;

use run_dpdk::offload::MbufTxOffload;
use run_dpdk::*;
use run_packet::ether::*;
use run_packet::ipv4::*;
use run_packet::udp::UdpPacket;
use run_packet::Buf;
use run_packet::CursorMut;
use run_packet::PktMut;

// The socket to work on
const WORKING_SOCKET: u32 = 1;
const THREAD_NUM: u32 = 4;
const START_CORE: usize = 33;

// dpdk batch size
const BATCH_SIZE: usize = 32;

// Basic configuration of the mempool
const MBUF_CACHE: u32 = 256;
const MBUF_NUM: u32 = MBUF_CACHE * 32 * THREAD_NUM;
const MP_NAME: &str = "wtf";

// Basic configuration of the port
const PORT_ID: u16 = 1;
const TXQ_DESC_NUM: u16 = 1024;
const RXQ_DESC_NUM: u16 = 1024;

// header info
const DMAC: [u8; 6] = [0x08, 0x68, 0x8d, 0x61, 0x6b, 0xf8];
const SMAC: [u8; 6] = [0x40, 0xa6, 0xb7, 0x60, 0xa2, 0xb1];
const DIP: [u8; 4] = [1, 1, 2, 2];
const SPORT: u16 = 60376;
const DPORT: u16 = 161;
const NUM_FLOWS: usize = 8192;

const PACKET_LEN: usize = 60;

static IP_ADDRS: OnceCell<Vec<[u8; 4]>> = OnceCell::new();

// range 2-251
// Generate at mot 62500 different IP addresses.
fn gen_ip_addrs(fst: u8, snd: u8, size: usize) -> Vec<[u8; 4]> {
    assert!(size <= 250 * 250);

    let mut v = Vec::new();

    for i in 0..size / 250 {
        for j in 2..252 {
            v.push([fst, snd, 2 + i as u8, j]);
        }
    }

    for j in 0..size % 250 {
        v.push([fst, snd, 2 + (size / 250) as u8, 2 + j as u8]);
    }

    v
}

fn entry_func() {
    IP_ADDRS.get_or_init(|| gen_ip_addrs(192, 168, NUM_FLOWS));

    // make sure that the rx and tx threads are on the correct cores
    let res = service()
        .lcores()
        .iter()
        .filter(|lcore| {
            lcore.lcore_id >= START_CORE as u32
                && lcore.lcore_id < START_CORE as u32 + THREAD_NUM as u32
        })
        .all(|lcore| lcore.socket_id == WORKING_SOCKET);
    assert!(res == true);

    let run = Arc::new(AtomicBool::new(true));
    let run_clone = run.clone();
    ctrlc::set_handler(move || {
        run_clone.store(false, Ordering::Release);
    })
    .unwrap();

    let mut jhs = Vec::new();

    for i in 0..THREAD_NUM as usize {
        let run_clone = run.clone();

        let jh = std::thread::spawn(move || {
            service().lcore_bind(i as u32 + START_CORE as u32).unwrap();

            let mut rxq = service().rx_queue(PORT_ID, i as u16).unwrap();
            let mut txq = service().tx_queue(PORT_ID, i as u16).unwrap();
            let mut batch = ArrayVec::<_, BATCH_SIZE>::new();
            let mut o_batch = ArrayVec::<_, BATCH_SIZE>::new();

            let mut tx_of_flag = MbufTxOffload::ALL_DISABLED;
            tx_of_flag.enable_ip_cksum();

            let ip_addrs = IP_ADDRS.get().unwrap();
            let mut adder: usize = 0;

            while run_clone.load(Ordering::Acquire) {
                rxq.rx(&mut batch);

                for mut mbuf in batch.drain(..) {
                    let buf = CursorMut::new(mbuf.data_mut());
                    if let Ok(mut ethpkt) = EtherPacket::parse(buf) {
                        if ethpkt.ethertype() == EtherType::IPV4 {
                            if let Ok(mut ippkt) = Ipv4Packet::parse(ethpkt.cursor_payload_mut()) {
                                if ippkt.protocol() == IpProtocol::UDP {
                                    if let Ok(mut udppkt) = UdpPacket::parse(ippkt.cursor_payload_mut()) {
                                        udppkt.set_dest_port(DPORT);
                                        udppkt.set_source_port(SPORT);
            
                                        ippkt.set_dest_ip(Ipv4Addr(DIP));
                                        ippkt.set_source_ip(Ipv4Addr(ip_addrs[adder % NUM_FLOWS]));
                                        adder += 1;
                                        let ip_hdr_len = ippkt.header_len();
                                        // ippkt.adjust_checksum();
            
                                        ethpkt.set_dest_mac(MacAddr(DMAC));
                                        ethpkt.set_source_mac(MacAddr(SMAC));
            
                                        mbuf.set_tx_offload(tx_of_flag);
                                        mbuf.set_l2_len(ETHER_HEADER_LEN as u64);
                                        mbuf.set_l3_len(ip_hdr_len as u64);
                                        
                                        o_batch.push(mbuf);
                                    }
                                }
                            }
                        }
                    }
                }

                while o_batch.len() > 0 {
                    // println!("try to send {} packets out", o_batch.len());
                    txq.tx(&mut o_batch);
                }
            }
        });
        jhs.push(jh);
    }

    let mut stats_query = service().stats_query(PORT_ID).unwrap();
    let mut old_stats = stats_query.query();
    let mut curr_stats = stats_query.query();
    while run.load(Ordering::Acquire) {
        std::thread::sleep(std::time::Duration::from_secs(1));
        stats_query.update(&mut curr_stats);
        println!(
            "rx: {} Mpps, {} Gbps || tx: {} Mpps, {} Gbps",
            (curr_stats.ipackets() - old_stats.ipackets()) as f64 / 1_000_000.0,
            (curr_stats.ibytes() - old_stats.ibytes()) as f64 * 8.0 / 1_000_000_000.0,
            (curr_stats.opackets() - old_stats.opackets()) as f64 / 1_000_000.0,
            (curr_stats.obytes() - old_stats.obytes()) as f64 * 8.0 / 1_000_000_000.0,
        );

        old_stats = curr_stats;
    }

    for jh in jhs {
        jh.join().unwrap();
    }
}

fn main() {
    DpdkOption::new().init().unwrap();

    // create mempool
    utils::init_mempool(MP_NAME, MBUF_NUM, MBUF_CACHE, WORKING_SOCKET).unwrap();

    // create the port
    utils::init_port(
        PORT_ID,
        THREAD_NUM as u16,
        THREAD_NUM as u16,
        RXQ_DESC_NUM,
        MP_NAME,
        TXQ_DESC_NUM,
        WORKING_SOCKET,
    )
    .unwrap();

    entry_func();

    // shutdown the port
    service().port_close(PORT_ID).unwrap();

    // free the mempool
    service().mempool_free(MP_NAME).unwrap();

    // shutdown the DPDK service
    service().service_close().unwrap();

    println!("dpdk service shutdown gracefully");
}
