use core::f64;
use std::{
    collections::VecDeque,
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    num::ParseFloatError,
};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, ParseError};
use csv::{Reader, StringRecord};
use csv_sniffer::error::SnifferError;
use eframe::Result;
use encoding_rs::*;
use encoding_rs_io::{DecodeReaderBytes, DecodeReaderBytesBuilder};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    functions::function::func_builder::FuncBuilder,
    types::{
        datetime_f64_mapper::{DateTimeF64Mapper, DateTimePrecision},
        point::Point,
    },
};

#[derive(Debug, PartialEq)]
pub enum ColumnType<'a> {
    DateTime(&'a str),
    Number,
}

pub struct Column<'a> {
    index: usize,
    typ: ColumnType<'a>,
}

pub struct Columns<'a, TIter: IntoIterator<Item = usize> + Clone> {
    x: Column<'a>,
    ys: TIter,
}

struct CSV<'a, Read, TIter: IntoIterator<Item = usize> + Clone> {
    reader: Reader<Read>,
    record: StringRecord,
    columns: Columns<'a, TIter>,
    headers: Vec<String>,
}

#[derive(Debug)]
pub enum ImporterError {
    IOError(io::Error),
    CSVError(csv::Error),
    SnifferError(SnifferError),
    ParseDateTimeError(ParseError),
    ParseFloatError(ParseFloatError),
    EmptyFileError(String),
}

impl From<io::Error> for ImporterError {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<SnifferError> for ImporterError {
    fn from(value: SnifferError) -> Self {
        Self::SnifferError(value)
    }
}

impl From<csv::Error> for ImporterError {
    fn from(value: csv::Error) -> Self {
        Self::CSVError(value)
    }
}

struct DecoderForSniffing {
    path: String,
    decoder: DecodeReaderBytes<BufReader<File>, Vec<u8>>,
    fallback_encoding: &'static Encoding,
}

impl DecoderForSniffing {
    fn init(
        path: &str,
        fallback: &'static Encoding,
    ) -> Result<DecodeReaderBytes<BufReader<File>, Vec<u8>>, io::Error> {
        // Open the file
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Create a decoder that attempts to detect encoding and converts it to UTF-8
        let decoder = DecodeReaderBytesBuilder::new()
            .encoding(None) // Let encoding_rs detect the encoding
            .bom_override(true)
            .bom_sniffing(true)
            .utf8_passthru(true)
            .encoding(Some(&fallback))
            .build(reader);
        Ok(decoder)
    }

    pub fn new(path: String, fallback: &'static Encoding) -> Result<Self, io::Error> {
        let decoder = Self::init(&path, fallback)?;
        Ok(Self {
            path,
            decoder,
            fallback_encoding: fallback,
        })
    }
}

impl io::Read for DecoderForSniffing {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.decoder.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.decoder.read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.decoder.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.decoder.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.decoder.read_exact(buf)
    }
}

impl Seek for DecoderForSniffing {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        if pos == SeekFrom::Start(0) {
            self.decoder = Self::init(&self.path, self.fallback_encoding)?;
        }
        Ok(0)
    }
}

#[derive(Debug, Clone)]
pub struct DateTimeInfo {
    pub format: String,
    pub mapper: DateTimeF64Mapper,
}

pub struct Importer {
    pub names: Vec<String>,
    pub mapper: Option<DateTimeInfo>,
}

impl Importer {
    /// Reads and decodes a file into UTF-8, using encoding detection.
    fn file_to_utf8_tester(file_path: &str) {
        let encodings: &[&Encoding] = &[
            BIG5,
            EUC_JP,
            EUC_KR,
            GB18030,
            GBK,
            IBM866,
            ISO_2022_JP,
            ISO_8859_10,
            ISO_8859_13,
            ISO_8859_14,
            ISO_8859_15,
            ISO_8859_16,
            ISO_8859_2,
            ISO_8859_3,
            ISO_8859_4,
            ISO_8859_5,
            ISO_8859_6,
            ISO_8859_7,
            ISO_8859_8,
            ISO_8859_8_I,
            KOI8_R,
            KOI8_U,
            SHIFT_JIS,
            UTF_8,
            UTF_16BE,
            UTF_16LE,
            WINDOWS_1250,
            WINDOWS_1251,
            WINDOWS_1252,
            WINDOWS_1253,
            WINDOWS_1254,
            WINDOWS_1255,
            WINDOWS_1256,
            WINDOWS_1257,
            WINDOWS_1258,
            WINDOWS_874,
            X_MAC_CYRILLIC,
        ];
        let mut data = String::new();
        // Open the file

        for encoding in encodings {
            let mut file = File::open(file_path).unwrap();
            let reader = BufReader::new(&mut file);

            // Create a decoder that attempts to detect encoding and converts it to UTF-8
            let mut decoder = DecodeReaderBytesBuilder::new()
                .encoding(None) // Let encoding_rs detect the encoding
                .encoding(Some(&encoding))
                .utf8_passthru(true)
                .build(reader);
            let _ = decoder.read_to_string(&mut data);
            println!("\n\n!!!!!!!Encoding!!!!!!: {:#?}\n", encoding.name());
            println!("{:#?}", data.chars().take(100).collect::<String>());
            data.clear();
        }
    }

    /// Reads and decodes a file into UTF-8, using encoding detection.
    fn read_file_to_utf8(
        file_path: String,
        fallback: &'static Encoding,
    ) -> Result<impl io::Read + io::Seek, io::Error> {
        DecoderForSniffing::new(file_path, fallback)
    }

    fn open_csv<'a>(
        path: String,
        mut buffer: StringRecord,
        column_matcher: &Regex,
        fallback_column_prefix: &str,
    ) -> Result<CSV<'a, impl io::Read, VecDeque<usize>>, ImporterError> {
        let mut rdr = Self::read_file_to_utf8(path.clone(), UTF_8)?;

        let mut sniffer = csv_sniffer::Sniffer::new();
        sniffer.sample_size(csv_sniffer::SampleSize::Records(200));

        let metadata = sniffer.sniff_reader(rdr)?;

        rdr = Self::read_file_to_utf8(path.clone(), UTF_8)?;
        let mut csv = metadata.dialect.open_reader(rdr)?;

        if csv.read_record(&mut buffer)? {
            let types_count = metadata.types.len();
            let mut x: Column = Column {
                index: types_count,
                typ: ColumnType::Number,
            };
            let mut ys = VecDeque::new();

            let headers = if csv.has_headers() {
                Some(csv.headers()?)
            } else {
                None
            };
            for (i, typ) in metadata.types.iter().enumerate() {
                match typ {
                    csv_sniffer::Type::Unsigned
                    | csv_sniffer::Type::Signed
                    | csv_sniffer::Type::Float => {
                        if let Some(headers) = headers {
                            if let Some(header) = headers.get(i) {
                                if column_matcher.is_match(header) {
                                    ys.push_back(i);
                                }
                            }
                        } else {
                            ys.push_back(i);
                        }
                    }
                    csv_sniffer::Type::Text => {
                        if x.index >= types_count {
                            if let Some(str) = buffer.get(i) {
                                match Self::parse_date(str) {
                                    Ok((_, format)) => {
                                        x.index = i;
                                        x.typ = ColumnType::DateTime(format)
                                    }
                                    Err(_) => (),
                                };
                            }
                        }
                    }
                    csv_sniffer::Type::Boolean => (),
                }
            }

            if x.index >= types_count && ys.len() > 1 {
                x.index = ys.pop_front().unwrap();
                x.typ = ColumnType::Number;
            }
            let headers: Vec<_> = if csv.has_headers() {
                match csv.headers() {
                    Ok(rec) => ys
                        .iter()
                        .map(|i| {
                            let v = rec.get(*i);
                            println!("{:#?}", v);
                            rec[*i].to_string()
                        })
                        .collect(),
                    Err(e) => {
                        return Err(ImporterError::CSVError(e));
                    }
                }
            } else {
                (0..(ys.len() + (if x.index >= types_count { 0 } else { 1 })))
                    .map(|x| format!("{} {}", fallback_column_prefix, x))
                    .collect::<Vec<_>>()
            };
            let meta = CSV {
                reader: csv,
                columns: Columns { x, ys },
                record: buffer,
                headers,
            };

            return Ok(meta);
        } else {
            return Err(ImporterError::EmptyFileError(path));
        }
    }

    fn parse_date_w_format(
        str: &str,
        format: &str,
        is_date_only_format: bool,
    ) -> Result<NaiveDateTime, ImporterError> {
        if is_date_only_format {
            match NaiveDate::parse_from_str(str, format) {
                Ok(date) => Ok(date.and_time(NaiveTime::MIN)),
                Err(e) => Err(ImporterError::ParseDateTimeError(e)),
            }
        } else {
            match NaiveDateTime::parse_from_str(str, format) {
                Ok(date) => Ok(date),
                Err(e) => Err(ImporterError::ParseDateTimeError(e)),
            }
        }
    }

    fn parse_date(str: &str) -> Result<(NaiveDateTime, &'static str), ImporterError> {
        const DATE_ONLY_FORMAT_INDEX: usize = 4;
        const FORMATS: [&str; 7] = [
            "%d.%m.%Y %H:%M:%S.%.f", // day, month, year, hour, minute, second, milliseconds
            "%d.%m.%Y %H:%M:%S",     // day, month, year, hour, minute, second
            "%d.%m.%Y %H:%M",        // day, month, year, hour, minute
            "%d.%m.%Y %H",           // day, month, year, hour
            "%d.%m.%Y",              // day, month, year
            "%m.%Y",                 // month, year
            "%Y",                    // year
        ];

        fn is_date_only_format(index: usize) -> bool {
            index >= DATE_ONLY_FORMAT_INDEX
        }

        fn get_format_index(str: &str) -> usize {
            fn u8_occur_count(bytes: &mut &[u8], byte: u8, max_count: usize) -> usize {
                let mut count = 0;
                while let Some(pos) = bytes.iter().position(|b| *b == byte) {
                    *bytes = &bytes[(pos + 1)..];
                    count += 1;
                    if count >= max_count {
                        break;
                    }
                }
                return count;
            }
            let mut bytes = str.as_bytes();
            let dot_count = u8_occur_count(&mut bytes, b'.', 2);
            return match dot_count {
                0 | 1 => FORMATS.len() - dot_count - 1,
                _ => {
                    let format_index = FORMATS.len() - dot_count - 1;
                    if let Some(space_pos) = bytes.iter().position(|b| *b == b' ') {
                        bytes = &bytes[(space_pos + 1)..].trim_ascii_start();
                        if bytes.len() > 0 {
                            let colon_count = u8_occur_count(&mut bytes, b':', 2);
                            return match colon_count {
                                0 | 1 => format_index - colon_count - 1,
                                _ => {
                                    format_index
                                        - colon_count
                                        - 1
                                        - u8_occur_count(&mut bytes, b'.', 1)
                                }
                            };
                        }
                    }
                    format_index
                }
            };
        }

        let format_index = get_format_index(str);
        let format = FORMATS[format_index];
        let is_date_only_format = is_date_only_format(format_index);
        Self::parse_date_w_format(str, format, is_date_only_format).map(|d| (d, format))
    }

    pub fn import(
        path: String,
        functions: &mut Vec<FuncBuilder>,
    ) -> Result<Importer, ImporterError> {
        static Y_MATCHER: Lazy<Regex> = Lazy::new(|| Regex::new(r"hodnota").unwrap());

        // Self::file_to_utf8_tester(path);

        let mut csv = Self::open_csv(path, StringRecord::new(), &Y_MATCHER, "Sloupec")?;
        functions.clear();
        for _i in 0..csv.columns.ys.len() {
            functions.push(FuncBuilder::new());
        }

        match Self::read_csv(&mut csv.reader, &mut csv.record, &csv.columns, functions) {
            Ok(mapper) => Ok(Importer {
                mapper,
                names: csv.headers,
            }),
            Err(e) => Err(e),
        }
    }

    fn read_csv<TIter>(
        r: &mut Reader<impl io::Read>,
        record: &mut StringRecord,
        columns: &Columns<TIter>,
        functions: &mut [FuncBuilder],
    ) -> Result<Option<DateTimeInfo>, ImporterError>
    where
        TIter: IntoIterator<Item = usize> + Clone,
    {
        let mut mapper = None;
        let mut is_eof = false;

        fn read_record(
            r: &mut Reader<impl io::Read>,
            record: &mut StringRecord,
        ) -> Result<bool, ImporterError> {
            match r.read_record(record) {
                Ok(is_not_eof) => Ok(!is_not_eof),
                Err(e) => {
                    return Err(ImporterError::CSVError(e));
                }
            }
        }

        fn parse_f64(str: &str) -> Result<f64, ImporterError> {
            match str.parse::<f64>() {
                Ok(x) => Ok(x),
                Err(e) => Err(ImporterError::ParseFloatError(e)),
            }
        }

        if record.is_empty() {
            is_eof = read_record(r, record)?;
        }
        let is_x_line_number =
            columns.x.typ == ColumnType::Number && columns.x.index >= record.len();
        let mut i = 0;
        while !is_eof {
            i += 1;
            let x: f64 = if is_x_line_number {
                i as f64
            } else {
                let x = &record[columns.x.index];
                match columns.x.typ {
                    ColumnType::DateTime(format) => {
                        let date = Self::parse_date(x)?.0;

                        let mapper = match &mapper {
                            None => &mapper
                                .insert(DateTimeF64Mapper::new(date, DateTimePrecision::Minutes)),
                            Some(mapper) => mapper,
                        };
                        mapper.time_to_f64(&date)
                    }
                    ColumnType::Number => parse_f64(x)?,
                }
            };
            for (y_str, func) in columns
                .ys
                .clone()
                .into_iter()
                .map(|c| &record[c])
                .zip(functions.iter_mut())
            {
                let y = parse_f64(y_str)?;
                func.add_point(&Point { x, y }).unwrap();
            }

            is_eof = read_record(r, record)?;
        }

        Ok({
            if let (Some(mapper), ColumnType::DateTime(format)) = (mapper, &columns.x.typ) {
                Some(DateTimeInfo {
                    mapper,
                    format: format.to_string(),
                })
            } else {
                None
            }
        })
    }
}
