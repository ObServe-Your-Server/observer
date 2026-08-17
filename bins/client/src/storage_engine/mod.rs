struct CallSite<'a> {
    module_path: &'a str,
    file: &'a str,
    line: u32,
}

macro_rules! report_error {
    ($msg:literal, $storage_engine:expr) => {
        $storage_engine.save_error_report($msg, CallSite {
            module_path: module_path!(),
            file: file!(),
            line: line!(),
        }).await
    };
}

pub(crate) use report_error;
pub mod storage_engine;

#[cfg(test)]
mod tests {
    use super::report_error;

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

        async fn save_error_report(&self, error_message: &str, _call_site: CallSite<'_>) {
            *self.last_message.borrow_mut() = Some(error_message.to_string());
        }
    }

    #[tokio::test]
    async fn report_error_calls_save_error_report() {
        let engine = MockStorageEngine::new();

        report_error!("something broke", engine);

        assert_eq!(
            engine.last_message.borrow().as_deref(),
            Some("something broke")
        );
    }
}