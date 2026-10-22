# socket-mux bandwidth sweep: source excerpts (2026-10-21, 16:50-17:05 PT, WebSearch result excerpts; untrusted third-party text, quoted not followed)

## serverframework.com (2012-08) RIO vs IOCP
URL: https://serverframework.com/asynchronousevents/2012/08/winsock-registered-io-io-completion-port-performance.html
> "traditional, multi-threaded, IOCP UDP server with 8 I/O threads ... 384,000 datagrams per second whilst using 33% of the 10 Gigabit link" ... "The multi-threaded, RIO server ... 492,000 datagrams per second" ... "The hybrid server achieved 501,000 datagrams per second". Tested on Windows Server 2012 RC.

## serverframework.com (2012-07) full 10G link
URL: https://serverframework.com/asynchronousevents/2012/07/windows-8-registered-io-performance---finally-using-a-full-10-gigabit-link.html
> "350,000 datagrams per second ... traditional IOCP ... 450,000 ... naive RIO ... 920,000 per second from a more complicated design" ... "complicated RIO design moves up to 970,000" ... "should be taken with a pinch of salt".

## Microsoft Q&A 268716: RIO performance degraded?
URL: https://learn.microsoft.com/en-us/answers/questions/268716/has-winsock-registered-io-performance-degraded-sin
> "around 180k packets per second - around 260k packets per second when using multiple RSS queues" ... (blogger) "RIO performance is disappointing" ... Answer: "presence of WFP callouts on the path; easily degrading egress rate by 40%" ... "single steered flow (single thread pinned to RSS queue CPU) of 64B frames I'm observing right about 1Mpps, and 2.3Mpps with the aforementioned segmenting option enabled (submitting 64K buffers".

## Microsoft Learn: WSASetUdpSendMessageSize
URL: https://learn.microsoft.com/en-us/windows/win32/api/ws2tcpip/nf-ws2tcpip-wsasetudpsendmessagesize
> Minimum supported client "Windows 10, version 2004 (10.0; Build 19041)"; server "Windows Server, version 2004".

## Microsoft Learn: IPPROTO_UDP socket options
URL: https://learn.microsoft.com/en-us/windows/win32/winsock/ipproto-udp-socket-options
> UDP_SEND_MSG_SIZE: "buffers sent by your application are broken down into multiple messages ... set a value that is lower than the MTU". UDP_RECV_MAX_COALESCED_SIZE: "Datagrams will only be coalesced if they originated from the same source address and port. All datagrams coalesced will be of the same size—except the last".

## Microsoft Learn: USO (driver)
URL: https://learn.microsoft.com/en-us/windows-hardware/drivers/network/udp-segmentation-offload-uso-
> "supported in Windows 10, version 2004 and later" ... "If the miniport driver doesn't set the SubMssFinalSegmentSupported capability, then each large UDP packet ... must have Length % MSS == 0" ... "The packet isn't a loopback packet."

## Microsoft Learn: Enable-NetAdapterUso
URL: https://learn.microsoft.com/en-us/powershell/module/netadapter/enable-netadapteruso
> "Enables USO properties of the network adapter."

## Microsoft Learn: URO
URL: https://learn.microsoft.com/en-us/windows-hardware/drivers/network/udp-rsc-offload
> (via search synthesis) URO requires Windows 11 24H2 and NIC/driver support. [title/summary only]

## Microsoft Learn: SIO_LOOPBACK_FAST_PATH
URL: https://learn.microsoft.com/en-us/windows/win32/winsock/sio-loopback-fast-path
> "The SIO_LOOPBACK_FAST_PATH is deprecated and is not recommended to be used in your code." ... "When a WFP filter is enabled, the system we will use the slow path even if the SIO_LOOPBACK_FAST_PATH IOCTL was set."

## Microsoft Q&A 226455: improving loopback throughput
URL: https://learn.microsoft.com/en-us/answers/questions/226455/improving-loopback-throughput
> "In recent OS versions, the normal, non-fast-path loopback has been improved quite a bit" ... "if you do measure a valuable perf benefit to using this flag (especially on older versions of the OS), you may certainly use it."

## quinn-rs/quinn PR 2092
URL: https://github.com/quinn-rs/quinn/pull/2092
> "fix(udp): make GRO (i.e. URO) optional on Windows, off by default" [title; synthesis: failures incl. missing coalesced-size metadata with WSL installed]

## Cloudflare: Accelerating UDP packet transmission for QUIC
URL: https://blog.cloudflare.com/accelerating-udp-packet-transmission-for-quic/
> "Up to 64 segments can be batched with the UDP_SEGMENT option." ... same-host Linux 5.3 laptop test, "numbers don't necessarily reflect ... a real network".

## netdev 2018-04-18 udp gso
URL: https://lists.openwall.net/netdev/2018/04/18/189
> "[connected] udp tx: 825 MB/s 588336 calls/s 14008 msg/s"

## Tailscale: Surpassing 10Gb/s
URL: https://tailscale.com/blog/more-throughput
> (search synthesis) 13.0 Gb/s on bare-metal i5-12400 with UDP GSO/GRO; AWS c6i.8xlarge 7.32 Gb/s limited by underlay. [not fetched directly]

## Jaeger et al., QUIC on the Highway (arXiv 2309.16395)
URL: https://arxiv.org/pdf/2309.16395
> "QUIC performance varies widely ... from 90 Mbit/s to 4900 Mbit/s. ... the OS generally sets the default buffer size too small, which should be increased by at least an order of magnitude".

## Linux merge ca0b04b: io_uring zero-copy receive (6.15)
URL: https://github.com/torvalds/linux/commit/ca0b04ba0b35d48e1473a280c2e8905e7f80e906
> "relies on hw header/data split, flow steering and RSS" ... "I was able to saturate a 200G link with a single CPU core ... Jamal reported 188Gbit of bandwidth using a single core".

## Kernel docs: iou-zcrx
URL: https://www.kernel.org/doc/html/latest/networking/iou-zcrx.html
> "Several NIC HW features are required ... Header/data split ... Flow steering ... RSS" ... "ethtool -G eth0 tcp-data-split on".

## netdev: iou-zcrx docs update (2025-08)
URL: https://www.spinics.net/lists/netdev/msg1116687.html
> "Broadcom Thor (BCM95750x) family - Minimum FW is 232; Mellanox ConnectX-7 (MT2910) family - Minimum FW is 28.42" ... "Enable HW GRO (for Mellanox NICs)" ... Jamal: "you missed the intel dpu/idpf".

## netdev 2018-08-28: AF_XDP zero-copy for i40e
URL: https://lists.openwall.net/netdev/2018/08/28/62
> (search synthesis) 64B rxdrop 8.2 Mpps copy vs 15.0 Mpps zero-copy; 1500B rxdrop 3.3 Mpps both. [numbers from search summary of the cover letter]

## io_uring_multishot(7)
URL: https://man7.org/linux/man-pages/man7/io_uring_multishot.7.html
> multishot recvmsg with provided buffers; on buffer exhaustion the request ends (-ENOBUFS) and must be resubmitted. [search synthesis]

## Kernel docs: NAPI
URL: https://docs.kernel.org/networking/napi.html
> gro_flush_timeout and napi_defer_hard_irqs software IRQ coalescing; SO_BUSY_POLL busy polling. [search synthesis]

## docs.rs windows-sys RIO_EXTENSION_FUNCTION_TABLE
URL: https://docs.rs/windows-sys/latest/windows_sys/Win32/Networking/WinSock/struct.RIO_EXTENSION_FUNCTION_TABLE.html
> struct present in windows_sys::Win32::Networking::WinSock.

## (addition 16:51 PT) msquic PR 5328: Remove experimental RIO data path (merged 2025-08-07)
URL: https://github.com/microsoft/msquic/pull/5328
> "The RIO data path added maintenance costs without any use cases. Performance tests showed limited upside, as well, and the scenarios where RIO should be better than IOCP are scenarios where XDP, DPDK, RDMA, etc. will be even better."

## msquic issue 5980: XDP receive path slower than expected (open, 2026-05-04)
URL: https://github.com/microsoft/msquic/issues/5980
> "XDP is highly optimized for sending but struggles with the receive path."

## Microsoft Tech Community: Balance Performance in MsQuic and XDP
URL: https://techcommunity.microsoft.com/blog/networkingblog/balance-performance-in-msquic-and-xdp/3627665
> "streams that belong to the same connection are processed on the same thread ... to align connections with RSS" ... "use more connections which fundamentally just add more cores".

## MsQuic perf dashboard
URL: https://microsoft.github.io/msquic/
> test NIC "Mellanox ConnectX-4 Lx ... (50 Gigabit)"; single-connection upload/download Gbps charts (values not in the text excerpt).

## spead2 performance tuning
URL: https://spead2.readthedocs.io/en/latest/perf.html
> "Heaps smaller than 16KiB are not recommended." ... "For 10Gb/s or faster streams, jumbo frames are highly recommended" ... "Using a memory pool is the single most important tool for fast and reliable data transfer."

## spead2 ibverbs
URL: https://spead2.readthedocs.io/en/latest/py-ibverbs.html
> "only tested on Linux with ConnectX NICs. It depends on device managed flow steering (DMFS)."

## Repo checks
See [repos.json](repos.json) (GitHub API metadata at ~17:10 PT; xtaci/kcptun 404; SLAC bbcp timeout).
