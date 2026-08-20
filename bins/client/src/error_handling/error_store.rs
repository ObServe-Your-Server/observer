use crate::error_handling::error::Error;

trait ErrorStore {
    fn save_error(error_message: Error) -> anyhow::Result<()>;
}
