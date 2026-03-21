// SPDX-License-Identifier: AGPL-3.0-only
//! Terminal and session configuration types for legacy systems

use serde::{Deserialize, Serialize};

/// Terminal types for interactive sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalType {
    /// VT100
    VT100,
    /// VT220
    VT220,
    /// VT320
    VT320,
    /// IBM 3270
    IBM3270,
    /// Tektronix 4010
    Tektronix4010,
    /// ANSI terminal
    ANSI,
    /// Dumb terminal
    Dumb,
    /// Custom terminal type.
    Custom {
        /// Terminal type name.
        name: String,
        /// Supported capabilities.
        capabilities: Vec<String>,
    },
}

/// Session configuration for interactive sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Terminal width
    pub width: u16,
    /// Terminal height
    pub height: u16,
    /// Line ending style
    pub line_ending: LineEnding,
    /// Character encoding
    pub encoding: CharacterEncoding,
    /// Flow control
    pub flow_control: FlowControl,
}

/// Line ending styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineEnding {
    /// Unix (LF)
    Unix,
    /// Windows (CRLF)
    Windows,
    /// Classic Mac (CR)
    ClassicMac,
    /// Custom line ending sequence.
    Custom {
        /// Line ending byte sequence.
        sequence: String,
    },
}

/// Character encodings for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharacterEncoding {
    /// ASCII
    ASCII,
    /// EBCDIC
    EBCDIC,
    /// UTF-8
    UTF8,
    /// ISO-8859-1
    ISO8859_1,
    /// CP437 (PC)
    CP437,
    /// PETSCII (Commodore)
    PETSCII,
    /// ATASCII (Atari)
    ATASCII,
    /// Custom character encoding.
    Custom {
        /// Encoding name.
        name: String,
    },
}

/// Flow control types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowControl {
    /// No flow control
    None,
    /// Hardware flow control (RTS/CTS)
    Hardware,
    /// Software flow control (XON/XOFF)
    Software,
}
