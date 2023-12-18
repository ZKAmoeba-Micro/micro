# CREATE

The EVM CREATE instructions are handled similarly.

For more information, see the
[micro documentation](https://micro.micro.io/docs/reference/architecture/differences-with-ethereum.html#create-create2).

## [CREATE](https://www.evm.codes/#f0?fork=shanghai)

[The LLVM IR generator code](https://github.com/ZKAmoeba-Micro/micro-compiler-llvm-context/blob/main/src/microvm/evm/create.rs#L19)
is common for Yul and EVMLA representations.

## [CREATE2](https://www.evm.codes/#f5?fork=shanghai)

[The LLVM IR generator code](https://github.com/ZKAmoeba-Micro/micro-compiler-llvm-context/blob/main/src/microvm/evm/create.rs#L57)
is common for Yul and EVMLA representations.
