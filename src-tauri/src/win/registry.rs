use crate::error::{Error, Result};
use crate::journal::{Hive, RegValue};
pub fn read(_h: Hive, _k: &str, _n: &str) -> Result<Option<RegValue>> { Err(Error::Unsupported) }
pub fn write(_h: Hive, _k: &str, _n: &str, _v: &RegValue) -> Result<()> { Err(Error::Unsupported) }
pub fn delete(_h: Hive, _k: &str, _n: &str) -> Result<()> { Err(Error::Unsupported) }
