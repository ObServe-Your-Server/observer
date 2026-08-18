pub struct CallSite<'a> {
    module_path: &'a str,
    file: &'a str,
    line: u32,
}

#[macro_export]
macro_rules! report_error {
    ($msg:literal, $storage_engine:expr) => {
        let call_site = CallSite {
            module_path: module_path!(),
            file: file!(),
            line: line!(),
        };
        log::error!("MSG: [{}] Module Path: [{}] File: [{}] Line: [{}]", $msg, call_site.module_path, call_site.file, call_site.line);

        $storage_engine.save_error_report($msg, CallSite {
            module_path: module_path!(),
            file: file!(),
            line: line!(),
        }).await
    };
}

#[cfg(test)]
mod tests {
    #[derive(Debug)]
    struct CallSite<'a> {
        module_path: &'a str,
        file: &'a str,
        line: u32,
    }

    struct MockStorageEngine {
        pub last_message: std::cell::RefCell<Option<String>>,
    }

    impl MockStorageEngine {
        fn new() -> Self {
            Self {
                last_message: std::cell::RefCell::new(None),
            }
        }

        async fn save_error_report(&self, error_message: &str, call_site: CallSite<'_>) {
            println!("Callsite: {:#?}", call_site);
            *self.last_message.borrow_mut() = Some(error_message.to_string());
        }
    }

    #[tokio::test]
    async fn report_error_calls_save_error_report() {
        let _ = env_logger::builder().is_test(true).try_init();

        let engine = MockStorageEngine::new();

        report_error!("something broke", engine);

        println!("Error: {:#?}", engine.last_message.borrow());

        assert_eq!(
            engine.last_message.borrow().as_deref(),
            Some("something broke")
        );
    }
}
