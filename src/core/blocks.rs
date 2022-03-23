use serde::{Deserialize, Serialize};

use super::header::Header;
use crate::core::{Config, Transaction};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block<C: Config> {
    pub header: Header<C>,
    pub body: Vec<Transaction>,
}

impl<C: Config> Block<C> {}
