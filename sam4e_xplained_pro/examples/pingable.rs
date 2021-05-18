#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use panic_semihosting as _; // panic handler
use sam4e_xplained_pro::{
    hal::{
        clock::*,
        ethernet,
        gpio::*,
        pac::{CorePeripherals, Peripherals},
        watchdog::*,
    },
    PHYADDRESS,
    Pins,
};

use smoltcp::wire::{Ipv4Address, IpCidr};
use smoltcp::iface::{InterfaceBuilder, Routes};
use smoltcp::socket::{SocketSet, SocketSetItem, RawSocketBuffer, RawPacketMetadata};
use smoltcp::time::Instant;
use smoltcp::dhcp::Dhcpv4Client;

// Number of preallocated descriptors for both receive and transmit.
const RXDESCRIPTOR_COUNT: usize = 8;
const TXDESCRIPTOR_COUNT: usize = 4;

static mut RXDESCRIPTORBLOCK: ethernet::RxDescriptorBlock<RXDESCRIPTOR_COUNT> = ethernet::RxDescriptorBlock::<RXDESCRIPTOR_COUNT>::const_default();
static mut TXDESCRIPTORBLOCK: ethernet::TxDescriptorBlock<TXDESCRIPTOR_COUNT> = ethernet::TxDescriptorBlock::<TXDESCRIPTOR_COUNT>::const_default();

#[entry]
fn main() -> ! {
    hprintln!("Network Pingable example started").ok();

    let _core = CorePeripherals::take().unwrap();
    let peripherals = Peripherals::take().unwrap();
    let clocks = ClockController::new(
        peripherals.PMC,
        &peripherals.SUPC,
        &peripherals.EFC,
        MainClock::RcOscillator12Mhz,
        SlowClock::RcOscillator32Khz,
    );
    let gpio_ports = Ports::new(
        (
            peripherals.PIOA,
            clocks.peripheral_clocks.pio_a.into_enabled_clock(),
        ),
        (
            peripherals.PIOB,
            clocks.peripheral_clocks.pio_b.into_enabled_clock(),
        ),
        (
            peripherals.PIOC,
            clocks.peripheral_clocks.pio_c.into_enabled_clock(),
        ),
        (
            peripherals.PIOD,
            clocks.peripheral_clocks.pio_d.into_enabled_clock(),
        ),
        (
            peripherals.PIOE,
            clocks.peripheral_clocks.pio_e.into_enabled_clock(),
        ),
    );
    let mut pins = Pins::new(gpio_ports, &peripherals.MATRIX);

    hprintln!("CPU Clock: {}", get_master_clock_frequency().0).ok();

    // Disable the watchdog timer.
    Watchdog::new(peripherals.WDT).disable();

    //
    // Ethernet controller setup
    //
    let eth = {
        unsafe {
            RXDESCRIPTORBLOCK.initialize(&peripherals.GMAC);
            TXDESCRIPTORBLOCK.initialize(&peripherals.GMAC);

            ethernet::Builder::new()
                .set_phy_address(PHYADDRESS)
                .freeze(
                    peripherals.GMAC, 
                    clocks.peripheral_clocks.gmac.into_enabled_clock(), 
                    pins.grefck,
                    pins.gtxen,
                    pins.gtx0,
                    pins.gtx1,
                    pins.gcrsdv,
                    pins.grx0,
                    pins.grx1,
                    pins.grxer,
                    pins.gmdc,
                    pins.gmdio,
                    &mut RXDESCRIPTORBLOCK,
                    &mut TXDESCRIPTORBLOCK)
        }
    };

    let mut ip_addrs = [IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0)];
    let mut routes_storage = [None; 1];
    let routes = Routes::new(&mut routes_storage[..]);
    
    // Create ethernet interface
    let mut interface = InterfaceBuilder::new(eth)
        .ip_addrs(&mut ip_addrs[..])
        .routes(routes)
        .finalize();

    let mut socket_storage:[Option<SocketSetItem>; 5] = [None, None, None, None, None,];
    let mut sockets = SocketSet::new(&mut socket_storage[..]);

    let mut dhcp_rx_metadata_buffer:[RawPacketMetadata; 1] = [RawPacketMetadata::EMPTY; 1];
    let mut dhcp_tx_metadata_buffer:[RawPacketMetadata; 1] = [RawPacketMetadata::EMPTY; 1];

    let mut dhcp_rx_payload_buffer: [u8; 900] = [0; 900];
    let mut dhcp_tx_payload_buffer: [u8; 600] = [0; 600];

    let dhcp_rx_buffer = RawSocketBuffer::new(
        &mut dhcp_rx_metadata_buffer[..],
        &mut dhcp_rx_payload_buffer[..]
    );
    let dhcp_tx_buffer = RawSocketBuffer::new(
        &mut dhcp_tx_metadata_buffer[..],
        &mut dhcp_tx_payload_buffer[..]
    );

    let mut dhcp = Dhcpv4Client::new(&mut sockets, dhcp_rx_buffer, dhcp_tx_buffer, Instant::from_millis(0));
///    let mut prev_cidr = Ipv4Cidr::new(Ipv4Address::UNSPECIFIED, 0);

    let mut previous_link_state = None;
    loop {
        let link_state = interface.device().link_state();
        if previous_link_state.is_none() || link_state != previous_link_state {
            if link_state.is_some() {
                hprintln!("Ethernet link is now UP with {} Mbps.", link_state.unwrap().0).unwrap();
            } else {
                hprintln!("Ethernet link is now DOWN.").unwrap();
            }

            previous_link_state = link_state;
        }

        if link_state.is_some() {
            let timestamp = Instant::from_millis(0);
            interface.poll(&mut sockets, timestamp)
                .map(|_| ())
                .unwrap_or_else(|e| hprintln!("Poll: {:?}", e).unwrap());

            let _config = dhcp.poll(&mut interface, &mut sockets, timestamp)
            .unwrap_or_else(|e| {
                hprintln!("DHCP: {:?}", e).unwrap();
                None
            });
       }
    }
}
