#![cfg(test)]

use libafl::observers::{AsanBacktraceObserver, ObserverWithHashField};

#[test]
fn rust_asan() {
    const ASAN_LOG: &str = r#"
    AddressSanitizer:DEADLYSIGNAL
    =================================================================
    ==1312799==ERROR: AddressSanitizer: ABRT on unknown address 0x45930014081f (pc 0x7ffff7c8ba6c bp 0x00000014081f sp 0x7fffffffdc00 T0)
        #0 0x7ffff7c8ba6c  (/lib64/libc.so.6+0x8ba6c) (BuildId: d78a44ae94f1d320342e0ff6c2315b2b589063f8)
        #1 0x7ffff7c3e685  (/lib64/libc.so.6+0x3e685) (BuildId: d78a44ae94f1d320342e0ff6c2315b2b589063f8)
        #2 0x7ffff7c28832  (/lib64/libc.so.6+0x28832) (BuildId: d78a44ae94f1d320342e0ff6c2315b2b589063f8)
        #3 0x555555561079  (/path/to/test/abort-test/target/x86_64-unknown-linux-gnu/debug/abort-test+0xd079) (BuildId: 1865aae6a5bd77739693d3e6a9c010512c8a0b81)

    ==1312799==Register values:
    rax = 0x0000000000000000  rbx = 0x00007ffff7eb87c0  rcx = 0x00007ffff7c8ba6c  rdx = 0x0000000000000006
    rdi = 0x000000000014081f  rsi = 0x000000000014081f  rbp = 0x000000000014081f  rsp = 0x00007fffffffdc00
     r8 = 0x00007fffffffdcd0   r9 = 0x0000000000000000  r10 = 0x0000000000000008  r11 = 0x0000000000000246
    r12 = 0x0000000000000006  r13 = 0x0000000000000001  r14 = 0x0000000000000000  r15 = 0x0000555555690000
    AddressSanitizer can not provide additional info.
    SUMMARY: AddressSanitizer: ABRT (/lib64/libc.so.6+0x8ba6c) (BuildId: d78a44ae94f1d320342e0ff6c2315b2b589063f8)
    ==1312799==ABORTING
    fish: Job 1, './target/x86_64-unknown-linux-g…' terminated by signal SIGABRT (Abort)
    "#;

    const ASAN_LOG_WO_REG: &str = r#"
    AddressSanitizer:DEADLYSIGNAL
    =================================================================
    ==1312799==ERROR: AddressSanitizer: ABRT on unknown address 0x45930014081f (pc 0x7ffff7c8ba6c bp 0x00000014081f sp 0x7fffffffdc00 T0)
        #0 0x7ffff7c8ba6c  (/lib64/libc.so.6+0x8ba6c) (BuildId: d78a44ae94f1d320342e0ff6c2315b2b589063f8)
        #1 0x7ffff7c3e685  (/lib64/libc.so.6+0x3e685) (BuildId: d78a44ae94f1d320342e0ff6c2315b2b589063f8)
        #2 0x7ffff7c28832  (/lib64/libc.so.6+0x28832) (BuildId: d78a44ae94f1d320342e0ff6c2315b2b589063f8)
        #3 0x555555561079  (/path/to/test/abort-test/target/x86_64-unknown-linux-gnu/debug/abort-test+0xd079) (BuildId: 1865aae6a5bd77739693d3e6a9c010512c8a0b81)

    AddressSanitizer can not provide additional info.
    SUMMARY: AddressSanitizer: ABRT (/lib64/libc.so.6+0x8ba6c) (BuildId: d78a44ae94f1d320342e0ff6c2315b2b589063f8)
    ==1312799==ABORTING
    "#;

    let mut obs = AsanBacktraceObserver::new("test");
    let empty_hash = obs.hash();
    obs.parse_asan_output(ASAN_LOG);
    let full_hash = obs.hash();
    assert_ne!(full_hash, empty_hash);
    obs.parse_asan_output(ASAN_LOG_WO_REG);
    let partial_hash = obs.hash();
    assert_eq!(partial_hash, full_hash);
}
