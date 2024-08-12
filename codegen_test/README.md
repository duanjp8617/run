## To build this crate we need:

1. ```cargo update -p ctrlc --precise 3.2.4```

2. ```cargo update -p home --precise 0.5.5```

## Dump the asm code

The following two commands will be super time-consuming. It also requires a special cargo command ```cargo-asm```.

1. ```cargo asm codegen_test::smol::handle_batch --asm-style=intel```

2. ```cargo asm codegen_test::run::handle_batch --asm-style=intel```