use crate::TokenId;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass(get_all, frozen))]
pub struct BestChoice {
    pub extra_token_ids: Vec<TokenId>,
    pub accum_log_prob: f64,
}

impl Default for BestChoice {
    fn default() -> Self {
        Self {
            extra_token_ids: Default::default(),
            accum_log_prob: f64::NEG_INFINITY,
        }
    }
}

impl BestChoice {
    pub fn update<S: IntoIterator<Item = TokenId>>(&mut self, token_ids: S, log_prob: f64) {
        if log_prob <= self.accum_log_prob {
            return;
        }
        self.accum_log_prob = log_prob;
        self.extra_token_ids = token_ids.into_iter().collect();
    }

    pub fn valid(&self) -> bool {
        self.accum_log_prob > f64::NEG_INFINITY
    }
}
