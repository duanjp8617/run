use arrayvec::ArrayVec;

use run_dpdk::{offload::MbufTxOffload, Mbuf};
use run_packet::*;

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
        let buf = CursorMut::new(mbuf.data_mut());
        if let Ok(mut ethpkt) = ether::EtherPacket::parse(buf) {
            if ethpkt.ethertype() == ether::EtherType::IPV4 {
                if let Ok(mut ippkt) = ipv4::Ipv4Packet::parse(ethpkt.cursor_payload_mut()) {
                    if ippkt.protocol() == ipv4::IpProtocol::UDP {
                        if let Ok(mut udppkt) = udp::UdpPacket::parse(ippkt.cursor_payload_mut()) {
                            udppkt.set_dest_port(DPORT);
                            udppkt.set_source_port(SPORT);

                            ippkt.set_dest_ip(ipv4::Ipv4Addr(DIP));
                            ippkt.set_source_ip(ipv4::Ipv4Addr(SIP));
                            let ip_hdr_len = ippkt.header_len();

                            ethpkt.set_dest_mac(ether::MacAddr(DMAC));
                            ethpkt.set_source_mac(ether::MacAddr(SMAC));

                            mbuf.set_tx_offload(tx_of_flag);
                            mbuf.set_l2_len(ether::ETHER_HEADER_LEN as u64);
                            mbuf.set_l3_len(ip_hdr_len as u64);
                            
                            obatch.push(mbuf);
                        }
                    }
                }
            }
        }
    }
}
