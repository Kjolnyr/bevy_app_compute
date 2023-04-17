pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    BufferNotFound(String),
    StagingBufferNotFound(String),
    PipelinesEmpty,
    PipelineNotReady,
    EncoderIsNone,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BufferNotFound(name) => write!(f, "Buffer {name} not found."),
            Error::StagingBufferNotFound(name) => write!(f, "Staging buffer {name} not found."),
            Error::PipelinesEmpty => {
                write!(f, "Missing pipelines. Have you added your shader plugins?")
            }
            Error::PipelineNotReady => write!(f, "Pipeline isn't ready yet."),
            Error::EncoderIsNone => write!(f, "The command encoder hasn't been initialized."),
        }
    }
}
