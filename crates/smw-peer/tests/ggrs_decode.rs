//! Seeded fuzzing of the GGRS packet decode: damaged, truncated and random packets from a peer never panic, and
//! no packet makes a decode allocate more than a fixed amount.

use smw_peer::{decode_ggrs_message, encode_ggrs_message};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;

thread_local! {
    static LIVE: Cell<usize> = const { Cell::new(0) };
    static PEAK: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = LIVE.try_with(|live| {
            live.set(live.get() + layout.size());
            let _ = PEAK.try_with(|peak| peak.set(peak.get().max(live.get())));
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = LIVE.try_with(|live| live.set(live.get().saturating_sub(layout.size())));
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// serde preallocates at most 1 MiB for a sequence, whatever its length prefix claims.
const MAX_DECODE_BYTES: usize = 1024 * 1024 + 64 * 1024;

/// Decodes `packet`, returning the message and the most bytes the decode held at once.
fn decode_measured(packet: &[u8]) -> (Option<ggrs::Message>, usize) {
    let base = LIVE.with(Cell::get);
    PEAK.with(|peak| peak.set(base));
    let msg = decode_ggrs_message(packet);
    (msg, PEAK.with(Cell::get) - base)
}

/// A packet as bincode lays it out: the magic, the body's variant index, then the body's fields.
fn packet(variant: u32, body: &[&[u8]]) -> Vec<u8> {
    let mut p = 0x5a3cu16.to_le_bytes().to_vec();
    p.extend(variant.to_le_bytes());
    for field in body {
        p.extend(*field);
    }
    p
}

/// One valid packet of every message body.
fn seeds() -> Vec<Vec<u8>> {
    let status = [
        &[0u8][..],
        &17i32.to_le_bytes(),
        &[1],
        &(-1i32).to_le_bytes(),
    ]
    .concat();
    vec![
        packet(0, &[&0xdead_beefu32.to_le_bytes()]),
        packet(1, &[&0xdead_beefu32.to_le_bytes()]),
        packet(
            2,
            &[
                &2u64.to_le_bytes(),
                &status,
                &[0],
                &20i32.to_le_bytes(),
                &18i32.to_le_bytes(),
                &3u64.to_le_bytes(),
                &[1, 2, 3],
            ],
        ),
        packet(3, &[&20i32.to_le_bytes()]),
        packet(4, &[&(-3i16).to_le_bytes(), &123_456u128.to_le_bytes()]),
        packet(5, &[&123_456u128.to_le_bytes()]),
        packet(6, &[&u128::MAX.to_le_bytes(), &60i32.to_le_bytes()]),
        packet(7, &[]),
    ]
}

/// Decodes `bytes` and checks the result: no panic, bounded allocation, and a decoded message encodes back to a
/// packet that decodes to the same message.
fn check(what: &str, bytes: &[u8]) -> bool {
    let (msg, peak) = decode_measured(bytes);
    assert!(
        peak <= MAX_DECODE_BYTES,
        "{what}: decoding {} bytes allocated {peak}",
        bytes.len()
    );
    if let Some(msg) = &msg {
        assert_eq!(
            decode_ggrs_message(&encode_ggrs_message(msg)).as_ref(),
            Some(msg),
            "{what}"
        );
    }
    msg.is_some()
}

#[test]
fn seeds_decode_and_round_trip() {
    for (i, seed) in seeds().iter().enumerate() {
        assert!(
            check(&format!("seed {i}"), seed),
            "seed {i} does not decode"
        );
        let msg = decode_ggrs_message(seed).unwrap();
        assert_eq!(&encode_ggrs_message(&msg), seed, "seed {i}");
    }
}

#[test]
fn truncated_packets_fail_to_decode() {
    for (i, seed) in seeds().iter().enumerate() {
        for cut in 0..seed.len() {
            assert!(
                !check(&format!("seed {i} cut at {cut}"), &seed[..cut]),
                "seed {i} cut at {cut} decoded"
            );
        }
    }
}

#[test]
fn huge_length_prefixes_stay_bounded() {
    let status_len = 5;
    for (len_at, len) in [
        (6, u64::MAX),
        (6, 1 << 40),
        (6 + 8 + 2 * status_len + 9, u64::MAX),
        (6 + 8 + 2 * status_len + 9, 1 << 40),
    ] {
        let mut bytes = seeds()[2].clone();
        bytes[len_at..len_at + 8].copy_from_slice(&len.to_le_bytes());
        assert!(!check(&format!("length {len} at {len_at}"), &bytes));
    }
}

#[test]
fn damaged_packets_decode_or_fail_cleanly() {
    let mut seed: u32 = 1;
    let mut next = move || {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (seed >> 8) as usize
    };
    let seeds = seeds();
    let mut decoded = 0;
    for case in 0..20_000 {
        let mut bytes = seeds[next() % seeds.len()].clone();
        match next() % 4 {
            0 => {
                for _ in 0..1 + next() % 4 {
                    let at = next() % bytes.len();
                    bytes[at] = next() as u8;
                }
            }
            1 => {
                let at = next() % bytes.len();
                let width = [1, 2, 4, 8][next() % 4].min(bytes.len() - at);
                bytes[at..at + width].fill(0xff);
            }
            2 => {
                let at = next() % (bytes.len() + 1);
                let extra: Vec<u8> = (0..next() % 16).map(|_| next() as u8).collect();
                bytes.splice(at..at, extra);
            }
            _ => bytes = (0..next() % 96).map(|_| next() as u8).collect(),
        }
        decoded += check(&format!("case {case}"), &bytes) as usize;
    }
    assert!(
        decoded > 0,
        "no damaged packet decoded, so the round trip was never checked"
    );
}
