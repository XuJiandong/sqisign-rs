Port this project's verifying part to ckb-vm. It targets rv64imc with the b extension, and no atomic instructions.

* Reference Rust options: https://github.com/nervosnetwork/ckb-script-templates/blob/main/contract/Makefile
* Use [ckb-std](https://github.com/nervosnetwork/ckb-std/blob/master/guide.md) when writing standalone apps.
* Support little-endian platforms only, as the target platform is little-endian.
* The verification part should support no_std.
* The signing part should support std.
* Not all KAT tests need to pass; focus on the verifying tests. Skip time-consuming tests.
* When running tests, use ckb-debugger (already installed): https://github.com/nervosnetwork/ckb-standalone-debugger/blob/develop/ckb-debugger/guide.md
* There is a similar project at /home/xjd/projects/shrincs-rs; when anything is uncertain, mimic it.

This job is done when a few smoke tests for the verifying part pass. No full test run is needed.

