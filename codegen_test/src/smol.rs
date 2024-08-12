use arrayvec::ArrayVec;

use run_dpdk::{offload::MbufTxOffload, Mbuf};
use smoltcp::wire;

// header info
const DMAC: [u8; 6] = [0x08, 0x68, 0x8d, 0x61, 0x6b, 0xf8];
const SMAC: [u8; 6] = [0x40, 0xa6, 0xb7, 0x60, 0xa2, 0xb1];
const DIP: [u8; 4] = [1, 1, 2, 2];
const SIP: [u8; 4] = [1, 1, 2, 2];
const SPORT: u16 = 60376;
const DPORT: u16 = 161;

#[inline(never)]
pub fn handle_batch(ibatch: &mut ArrayVec<Mbuf, 32>, obatch: &mut ArrayVec<Mbuf, 32>) {
    let mut tx_of_flag = MbufTxOffload::ALL_DISABLED;
    tx_of_flag.enable_ip_cksum();
    for mut mbuf in ibatch.drain(..) {
        if let Ok(mut ethpkt) = wire::EthernetFrame::new_checked(mbuf.data_mut()) {
            if ethpkt.ethertype() == wire::EthernetProtocol::Ipv4 {
                if let Ok(mut ippkt) = wire::Ipv4Packet::new_checked(ethpkt.payload_mut()) {
                    if ippkt.protocol() == wire::IpProtocol::Udp {
                        if let Ok(mut udppkt) = wire::UdpPacket::new_checked(ippkt.payload_mut()) {
                            udppkt.set_dst_port(DPORT);
                            udppkt.set_src_port(SPORT);

                            ippkt.set_dst_addr(wire::Ipv4Address(DIP));
                            ippkt.set_src_addr(wire::Ipv4Address(SIP));
                            let ip_hdr_len = ippkt.header_len();

                            ethpkt.set_dst_addr(wire::EthernetAddress(DMAC));
                            ethpkt.set_src_addr(wire::EthernetAddress(SMAC));

                            mbuf.set_tx_offload(tx_of_flag);
                            mbuf.set_l2_len(14 as u64);
                            mbuf.set_l3_len(ip_hdr_len as u64);

                            obatch.push(mbuf);
                        }
                    }
                }
            }
        }
    }
}
