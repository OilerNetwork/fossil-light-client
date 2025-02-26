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
