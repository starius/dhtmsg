## Plan
- Pick a Rust BitTorrent DHT crate (for announce/get_peers) and map the PoC flow to it.
- Build a tiny UDP demo: use random hex ID, derive infohash (SHA-1), announce self, query peer infohash, send/receive Hello over UDP.
- Smoke-test locally (best-effort), note limits/gaps, and report how to run it.
