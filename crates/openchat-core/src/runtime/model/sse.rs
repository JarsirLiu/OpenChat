use futures_util::{Stream, StreamExt};

use crate::ChatServiceError;

pub struct SseDataSource<S> {
    inner: S,
    pending_bytes: Vec<u8>,
    pending_data_lines: Vec<String>,
}

impl<S> SseDataSource<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            pending_bytes: Vec::new(),
            pending_data_lines: Vec::new(),
        }
    }
}

impl<S> SseDataSource<S>
where
    S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
{
    pub async fn next_data(&mut self) -> Result<Option<String>, ChatServiceError> {
        loop {
            if let Some(payload) = self.drain_next_event()? {
                return Ok(Some(payload));
            }

            match self.inner.next().await {
                Some(Ok(chunk)) => self.pending_bytes.extend_from_slice(&chunk),
                Some(Err(error)) => return Err(ChatServiceError::new(502, error.to_string())),
                None => {
                    if let Some(payload) = self.finish_trailing_event()? {
                        return Ok(Some(payload));
                    }
                    return Ok(None);
                }
            }
        }
    }

    fn drain_next_event(&mut self) -> Result<Option<String>, ChatServiceError> {
        while let Some(newline_index) = self.pending_bytes.iter().position(|byte| *byte == b'\n') {
            let mut line_bytes = self
                .pending_bytes
                .drain(..=newline_index)
                .collect::<Vec<_>>();
            if matches!(line_bytes.last(), Some(b'\n')) {
                line_bytes.pop();
            }
            if matches!(line_bytes.last(), Some(b'\r')) {
                line_bytes.pop();
            }

            let line = std::str::from_utf8(&line_bytes).map_err(|error| {
                ChatServiceError::new(502, format!("invalid SSE utf-8 line: {error}"))
            })?;

            if line.is_empty() {
                if self.pending_data_lines.is_empty() {
                    continue;
                }

                let payload = self.pending_data_lines.join("\n").trim().to_string();
                self.pending_data_lines.clear();
                if payload.is_empty() {
                    continue;
                }
                return Ok(Some(payload));
            }

            if let Some(rest) = line.strip_prefix("data:") {
                self.pending_data_lines.push(rest.trim_start().to_string());
            }
        }

        Ok(None)
    }

    fn finish_trailing_event(&mut self) -> Result<Option<String>, ChatServiceError> {
        if !self.pending_bytes.is_empty() {
            let trailing = std::str::from_utf8(&self.pending_bytes).map_err(|error| {
                ChatServiceError::new(502, format!("invalid trailing SSE utf-8: {error}"))
            })?;
            for line in trailing.lines() {
                if let Some(rest) = line.strip_prefix("data:") {
                    self.pending_data_lines.push(rest.trim_start().to_string());
                }
            }
            self.pending_bytes.clear();
        }

        if self.pending_data_lines.is_empty() {
            return Ok(None);
        }

        let payload = self.pending_data_lines.join("\n").trim().to_string();
        self.pending_data_lines.clear();
        if payload.is_empty() {
            return Ok(None);
        }

        Ok(Some(payload))
    }
}
