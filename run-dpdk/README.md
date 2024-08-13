# Run Testcases
1. Add the following lines to `~/.cargo/config`:
```shell
[target.x86_64-unknown-linux-gnu]
runner = 'sudo -E'
```

2. The testcases must be run within a single thread, so use the following command to run the test:
```shell
cargo test -- --test-threads=1
```

# Examples

The contained examples are used to test the correctness of the implementation of various DPDK-related features. 

## Loopback speed test: 

Use two servers, one launches the ```loopback_tx.rs``` to generate traffic. The other one launches the ```looback_rx.rs``` to receive traffic and loop the traffic back at the same port. 

## Rss test:

1. Generate traffic with ```loopback_tx.rs```.
2. Receive traffic with ```rss_rx.rs```. This program will print the number of IP/UDP flows received from each queue in each second. 

## Checksum offload test for mbuf:

1. Generate traffic with ```checksum_offload_tx.rs```. Note that this example accepts a command line argument in the range of 0-5 (4/5 generate a UDP packet with trailing unused bytes), and generates different kinds of traffic depending on this argument.
2. Receive the traffic with ```checksum_offload_rx.rs```
3. This example can be run with ```multiseg``` feature enabled.

For instance, if we add trailing unused bytes to the end of the UDP packet, while the tx offloading can correctly calculate the checksum value, the rx checksum will report invalid checksum. This is an interesting finding.


## Jumboframe test:
1. If we use Mellanox NIC, we need to first set the mtu to 9000 with the following command:
```shell
ip link set dev NAME mtu 9000
```
The ```NAME``` refers the interface name in the Linux kernel, it can be checked with DPDK's ```dpdk-devbind.py``` script. 

2. 

# DPDK devices

## Intel E810

1. Configure huge page and iommu by adding the following line to the ```GRUB_CMDLINE_LINUX``` field of the ```/etc/default/grub```
```shell
GRUB_CMDLINE_LINUX="intel_iommu=on iommu=pt hugepagesz=1G hugepages=16 default_hugepagesz=1G intel_pstate=disable"
``` 

2. Run the following command to apply the change in the grub system:
```shell
grub-mkconfig -o /boot/grub/grub.cfg
```

3. Reboot the system, and check whether the kernel parameters take effect with the following command:
```shell
cat /proc/cmdline
```

## Intel E810 Driver Compatibility

It seems that to correctly use Intel E810 driver, we need to install the correct device driver version. 
It seems that we need upgrade E810 firmware, upgrade Linux ICE driver version, and update the Intel DDP version. 

Note:

By just upgrading the ICE driver to 1.11.14, we fix this problem. 
Go to intel official site
```shell
https://www.intel.com/content/www/us/en/download/19630/intel-network-adapter-driver-for-e810-series-devices-under-linux.html
```
And download the latest ICE driver version.

Untar the driver, switch to /src folder, issue the following commands:
```shell
make -j
sudo make install
sudo rmmod ice
sudo modprobe ice
```

To check whether ice driver version is correctly upgraded, use the following command:
```shell
modinfo ice
sudo ethtool -i DEVICE_NAME
```

Updating Firmware:

Add details on how to update firmware

## Rustc version

Currently we should use rust version 1.68.0 to get the significant leading performance.

To compile with 1.68.0, one needs to first switch the toolchain to 1.68.0 with:
```shell
rustup default 1.68.0
```

Then, during compilation, some potential dependency errors may occur, we can fix them with:
```shell
cargo update -p nix@0.28.0 --precise 0.26.4
cargo update -p ctrlc --precise 3.2.4
cargo update -p home --precise 0.5.5
```

