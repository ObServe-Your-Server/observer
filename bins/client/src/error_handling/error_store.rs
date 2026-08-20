use crate::error_handling::error::Error;
use anyhow::Result;

trait ErrorStore {
    fn save_error(error_message: Error) -> Result<()>;
}
