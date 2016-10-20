OCS server dummy
================

This project implements a small dummy OCS (Online Charging System) server for
performance testing of the Gy protocol. Basically it's just a tiny server that
accepts TCP connections, parses the minimal amount needed for each request, and
replies as fast as possible with some well-formed data.

Goals
-----
* To be able to saturate a 1G Ethernet link.

Non-goals
---------
* Use case/scenario/compliance testing of any kind.

Specifications
--------------
Diameter base protocol: [RFC 6733](https://tools.ietf.org/html/rfc6733)

Credit control application: [RFC 4006](https://tools.ietf.org/html/rfc4006)

Gy protocol: 3GPP TS 32.299 (overrides RFC 4006 in some places).

Compiling
---------
This program is written in Rust, which is super cool. Get your copy from your local
[dealer](https://www.rust-lang.org) today(tm) and compile with:

     cargo build

