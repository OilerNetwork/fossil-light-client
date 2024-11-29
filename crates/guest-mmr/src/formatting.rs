use thiserror::Error;

#[derive(Clone)]
pub struct FormattingOptions {
    pub output_size: usize,
    pub null_value: String,
}

pub type ProofFormattingOptions = FormattingOptions;
pub type PeaksFormattingOptions = FormattingOptions;

#[derive(Clone)]
pub struct FormattingOptionsBundle {
    pub proof: ProofFormattingOptions,
    pub peaks: PeaksFormattingOptions,
}

#[derive(Clone, Default)]
pub struct ProofOptions {
    pub elements_count: Option<usize>,
    pub formatting_opts: Option<FormattingOptionsBundle>,
}

#[derive(Error, Debug)]
pub enum FormattingError {
    #[error("Formatting: Expected peaks output size is smaller than the actual size")]
    PeaksOutputSizeError,
    #[error("Formatting: Expected proof output size is smaller than the actual size")]
    ProofOutputSizeError,
}
