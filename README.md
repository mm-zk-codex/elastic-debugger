# Elastic chain debugger

This is a small tool to help debug the localhost deployments of the elastic chain.


When executed - it will do a quick scan of the local networks deployed, and provide the basic information.

It assumes that the L1 is running on port 8545, Gateway on port 3050, and then client chain on 3060.

```
cargo run
```

Here's the example output from the tool:
```
====================================
=====   Elastic chain debugger =====
====================================
[OK] L1 (ethereum) - Sequencer at http://127.0.0.1:8545 (Chain: 9, Last Block: 338387), L1
[OK] L2 (gateway)  - Sequencer at http://127.0.0.1:3050 (Chain: 270, Last Block: 63), L2 -> 9
[ERROR] L3 (client)   - Port not active: http://127.0.0.1:3060
===
=== Bridehubs
===
Found 2 chains on L1 bridgehub: Some({270, 320})
Contracts on L1:
  Bridgehub:          0x9cAC3E80223AF3aF00d591e53336CBe05953c0a0
  Chain: 270
    Shared bridge:      0x817C5c088078AE9DDAc1EEa2f9bb843E09aa5Eba
    STM:                0x4eD263cD49cD3B111D6cf15214c8C40114e9Fd94
    ST:                 0x26C6BcaD82f0305F58445f417e80B49AcC2373f4
    Base Token:         0x0000000000000000000000000000000000000001
    Validator timelock: 0x743fCf7e4830a05C1a3E103301Aa92D15Cdc6d8f
    STM Asset id:       0xbd4f8412fad59106b1808a3cc0c21dd7b28ea9de4f1fb44fed6ba2d6cafdf726

  Chain: 320
    Shared bridge:      0x817C5c088078AE9DDAc1EEa2f9bb843E09aa5Eba
    STM:                0x4eD263cD49cD3B111D6cf15214c8C40114e9Fd94
    ST:                 0x34c531811184cd4862188d475387308Db003a5Dc
    Base Token:         0x0000000000000000000000000000000000000001
    Validator timelock: 0x743fCf7e4830a05C1a3E103301Aa92D15Cdc6d8f
    STM Asset id:       0xbd4f8412fad59106b1808a3cc0c21dd7b28ea9de4f1fb44fed6ba2d6cafdf726

Found 1 chains on Gateway bridgehub: Some({320})
L2 contracts on Gateway:
  Bridgehub:          0x0000000000000000000000000000000000010002
  Chain: 320
    Shared bridge:      0x0000000000000000000000000000000000010003
    STM:                0x5db38EF672d53aa5b09cCc29170154958b4BD81f
    ST:                 0x7aF6daF703ce77FD02bBc50687b6497863653D5c
    Base Token:         0x0000000000000000000000000000000000000000
    Validator timelock: 0xBE5FfF81acCe5626e89c75252C24985BE4F4E930
    STM Asset id:       0x1c2afac0e7e2d2746a54dc2d8ab8b622e4dfda07641d1e95d411ac0ce087b48a

===
=== State Transitions
===
Chain 270 on L1: Chain id: 270
  Protocol version: 0.25.1
  Batches (C,V,E):  19 19 19
  System upgrade:   0x0000000000000000000000000000000000000000000000000000000000000000
  AA hash:          0x0100055de356de05b75c83195567a6688d9050a17b58ccc5c5c91d05cd2bfb6d
  Verifier:         0x8E6356A6F8698a9e83624eD5c504a6953aEC41A2
  Admin:            0xED8E5051FA4EF5Ed72bD6E397d7a03547Aabd35C
  Bootloader hash:  0x010008eb70b467979695d3f240d8db04b1b179dd02c0d7fd45a027fb4bd9ecaf
  Sync layer:       0x0000000000000000000000000000000000000000

Chain 320 on L1: Chain id: 320
  Protocol version: 0.25.1
  Batches (C,V,E):  1 1 1
  System upgrade:   0x0000000000000000000000000000000000000000000000000000000000000000
  AA hash:          0x0100055de356de05b75c83195567a6688d9050a17b58ccc5c5c91d05cd2bfb6d
  Verifier:         0x8E6356A6F8698a9e83624eD5c504a6953aEC41A2
  Admin:            0xED8E5051FA4EF5Ed72bD6E397d7a03547Aabd35C
  Bootloader hash:  0x010008eb70b467979695d3f240d8db04b1b179dd02c0d7fd45a027fb4bd9ecaf
  Sync layer:       0x26C6BcaD82f0305F58445f417e80B49AcC2373f4

Chain 320 on Gateway: Chain id: 320
  Protocol version: 0.25.1
  Batches (C,V,E):  0 0 0
  System upgrade:   0x9a57cfc833eb70507999221c7103460390084807800398c2ab7f76caed6b4920
  AA hash:          0x0100055de356de05b75c83195567a6688d9050a17b58ccc5c5c91d05cd2bfb6d
  Verifier:         0x8361AC78EdCf79136dFa5ebC04F78352761Be45D
  Admin:            0x52312AD6f01657413b2eaE9287f6B9ADaD93D5FE
  Bootloader hash:  0x010008eb70b467979695d3f240d8db04b1b179dd02c0d7fd45a027fb4bd9ecaf
  Sync layer:       0x0000000000000000000000000000000000000000
```