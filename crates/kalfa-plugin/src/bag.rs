//! WIT 0.1'in host tarafı bağları (`kalfa-plugin-sdk/wit/kalfa.wit`).

wasmtime::component::bindgen!({
    world: "eklenti",
    path: "../kalfa-plugin-sdk/wit",
    imports: { default: async },
    exports: { default: async },
});
