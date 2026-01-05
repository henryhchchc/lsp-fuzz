# LSPFuzz: Hunting Bugs in Language Servers

LSPFuzz is a grey-box hybrid fuzzer that generates test cases for [Language Servers](https://microsoft.github.io/language-server-protocol/).
It is implemented based on [LibAFL](https://github.com/AFLplusplus/LibAFL).

## What is this?

> The language server crashed five times in the past three minutes.

> The code completions are suddenly gone when I was typing.

Sound familiar? It should! Bugs in language servers can cause interruptions in your development workflow, even when you haven't done anything wrong. LSPFuzz is designed to automatically find such bugs before they are shipped to you.

## Technical Details

LSPFuzz is equipped with a two-stage mutation pipeline that produces valid yet diverse inputs to trigger various analysis routines in LSP servers.
To learn more about how it works, please check out the following research paper:

Hengcheng Zhu, Songqiang Chen, Valerio Terragni, Lili Wei, Yepang Liu, Jiarong Wu, and Shing-Chi Cheung.
**LSPFuzz: Hunting Bugs in Language Servers.**
In _Proceedings of the 40<sup>th</sup> IEEE/ACM International Conference on Automated Software Engineering._ Seoul, South Korea. November 2025.

[🔗 DOI](https://doi.org/10.1109/ASE63991.2025.00183)
| [🎤 Conference](https://conf.researchr.org/details/ase-2025/ase-2025-papers/203/LSPFuzz-Hunting-Bugs-in-Language-Servers)
| [📄 Preprint](https://scholar.henryhc.net/files/publications/2025/ASE2025-LSPFuzz.pdf)
| [📦 Artifacts](https://doi.org/10.5281/zenodo.17052142)

If you use LSPFuzz for academic purposes, please cite the above paper.
A snapshot of the code used to conduct the experiments in the paper can be found at the [ase25-major-revision](https://github.com/henryhchchc/lsp-fuzz/releases/tag/ase25-major-revision) tag.

## Usage

### Preparation

1. Prepare a fuzz target compatible with [AFL++](https://github.com/AFLplusplus/AFLplusplus).
   It is highly recommended to use [LTO mode](https://github.com/AFLplusplus/AFLplusplus/blob/stable/instrumentation/README.lto.md) and [persistent mode](https://github.com/AFLplusplus/AFLplusplus/blob/stable/instrumentation/README.persistent_mode.md).
   The following is an annotated template for a fuzz target:

   ```c++
   #include "your_header_file.h"

   #ifndef __AFL_FUZZ_TESTCASE_LEN
       // The following definitions allow compilation without the AFL++ compiler.
       ssize_t fuzz_len;
       #define __AFL_FUZZ_TESTCASE_LEN fuzz_len
       const uint8_t fuzz_buf[1024000];
       #define __AFL_FUZZ_TESTCASE_BUF fuzz_buf
       #define __AFL_FUZZ_INIT() void sync(void);
       #define __AFL_LOOP(x) ((fuzz_len = read(0, fuzz_buf, sizeof(fuzz_buf))) > 0 ? 1 : 0)
       #define __AFL_INIT() sync()
   #endif

   __AFL_FUZZ_INIT();

   int main(int argc, const char* argv[]) {

       #ifdef __AFL_HAVE_MANUAL_CONTROL
         __AFL_INIT();
       #endif

       // [Initialization]
       // Perform one-time initialization for the target LSP server.
       // Or call `LLVMFuzzerInitialize(argc, argv)` here.

       const uint8_t *buf = __AFL_FUZZ_TESTCASE_BUF;
       while (__AFL_LOOP(10000)) {
           ssize_t len = __AFL_FUZZ_TESTCASE_LEN;
           // [Input Processing]
           // Process an input here:
           //   1. Read `len` bytes from `buf` for LSP inputs, as if they were read from `stdin`.
           //   2. Process the LSP inputs. Note that the input contains the `Content-Length` header.
           //   3. Release resources and reset states.
           // Or call `LLVMFuzzerTestOneInput(buf, len)` here.
       }
       return 0;
   }
   ```

2. Obtain the coverage map size:

   ```bash
   AFL_DUMP_MAP_SIZE=1 ./fuzz-target
   ```

3. Mine code fragments for code generation:

   ```bash
   lsp-fuzz-cli mine-code-fragments \
     --search-directory <code-dir> \ # Directory containing code files of the target language for the LSP servers
     --output <fragment-output> # File to store the mined code fragments
   ```

> [!CAUTION]
> Although persistent mode can significantly improve fuzzing efficiency, users need to ensure that resources are properly released and states are reset in the fuzzing loop.

### Start Fuzzing

```bash
lsp-fuzz-cli fuzz \
  --state <state-dir> \ # Directory to store the fuzzing state (e.g., generated inputs, found crashes)
  --lsp-executable <fuzz-target> \ # Executable file of the LSP server fuzz target
  --language-fragments Language=<fragment-output>\ # Comma-separated list of files containing the mined code fragments, (e.g., `C=c.frag,CPlusPlus=cpp.frag`)
  --coverage-map-size <coverage-map-size> \ # Size of the coverage map to use for coverage-guided fuzzing
  --time-budget 24 # Time budget for fuzzing in hours
```

To learn more about the options, run `lsp-fuzz-cli fuzz --help`.

### Reproduce Detected Crashes

1. Export the generated crash-triggering inputs:

   ```bash
   lsp-fuzz-cli export \
     --input <state-dir>/solutions \ # Directory containing the generated crash-triggering inputs
     --output <export-directory> # Directory to store the exported crash-triggering inputs
   ```

   The contents of `<export-directory>` will be organized as follows:

   ```
   <export-directory>
   ├── <input-id-0>
   │   ├── workspace
   │   │   ├── file1.txt
   │   │   └── file2.txt
   │   └── requests
   │       ├── message_0001
   │       └── message_0002
   ├── <input-id-1>
   │   ├── workspace
   │   │   ├── file1.txt
   │   │   └── file2.txt
   │   └── requests
   │       ├── message_0001
   │       └── message_0002
   └── ...
   ```

   Each directory `<input-id>` represents a unique input generated by LSPFuzz.
   Within each `<input-id>` directory, there are two subdirectories: `workspace` and `requests`.
   The `workspace` directory contains the code files, and the `requests` directory contains the LSP requests that were sent to the LSP server during fuzzing.

2. Feed the exported input to the LSP server:

   To reproduce the crash, `cd` to a directory containing the exported inputs.

   ```bash
   cat requests/* | ./target-lsp-server
   ```

   Note that `target-lsp-server` is the actual LSP server under test, not the fuzz target.
   Make sure it reads requests from `stdin` and the CLI options are properly set.
   To reproduce bugs caught by sanitizers, `target-lsp-server` should be compiled with sanitizers enabled.

> [!IMPORTANT]
> Do not move the exported test cases, because the LSP requests are encoded with _absolute paths_. Moving them will invalidate the requests (analogous to the concept of [pinning](https://doc.rust-lang.org/std/pin/index.html) in Rust).
