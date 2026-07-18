use shaku::module;

module! {
    pub AppModule {
        components = [],
        providers = []
    }
}

pub fn test_module() -> AppModule {
    AppModule::builder().build()
}
