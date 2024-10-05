use std::{fs::File, num::ParseFloatError};

use chrono::{NaiveDateTime, ParseError};
use csv::{Reader, StringRecord};
use eframe::Result;

use crate::{
    functions::function::func_builder::FuncBuilder,
    types::{
        datetime_f64_mapper::{DateTimeF64Mapper, DateTimePrecision},
        point::Point,
    },
};

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

#[derive(Debug)]
pub enum ImporterError {
    CSVError(csv::Error),
    ParseDateTimeError(ParseError),
    ParseFloatError(ParseFloatError),
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
    pub fn import(path: &str, functions: &mut Vec<FuncBuilder>) -> Result<Importer, ImporterError> {
        let columns = Columns {
            x: Column {
                index: 4,
                typ: ColumnType::DateTime("%d.%m.%Y %H:%M"),
            },
            ys: (0..16).map(|i| i * 2 + 5),
        };
        let mut csv = match csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .from_path(path)
        {
            Ok(csv) => csv,
            Err(e) => {
                return Err(ImporterError::CSVError(e));
            }
        };
        let headers: Vec<_> = match csv.headers() {
            Ok(rec) => columns
                .ys
                .clone()
                .into_iter()
                .map(|i| rec[i].to_string())
                .collect(),
            Err(e) => {
                return Err(ImporterError::CSVError(e));
            }
        };
        functions.clear();
        for _i in 0..headers.len() {
            functions.push(FuncBuilder::new());
        }

        let mut record = StringRecord::new();
        match Self::read_csv(&mut csv, &mut record, &columns, functions) {
            Ok(mapper) => Ok(Importer {
                mapper,
                names: headers,
            }),
            Err(e) => Err(e),
        }
    }

    fn read_csv<TIter>(
        r: &mut Reader<File>,
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
            r: &mut Reader<File>,
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

        fn parse_date(str: &str, format: &str) -> Result<NaiveDateTime, ImporterError> {
            match NaiveDateTime::parse_from_str(str, format) {
                Ok(date) => Ok(date),
                Err(e) => Err(ImporterError::ParseDateTimeError(e)),
            }
        }

        if record.is_empty() {
            is_eof = read_record(r, record)?;
        }
        while !is_eof {
            let x = &record[columns.x.index];
            let x = match columns.x.typ {
                ColumnType::DateTime(format) => {
                    let date = parse_date(x, format)?;

                    let mapper = match &mapper {
                        None => {
                            &mapper.insert(DateTimeF64Mapper::new(date, DateTimePrecision::Minutes))
                        }
                        Some(mapper) => mapper,
                    };
                    mapper.time_to_f64(&date)
                }
                ColumnType::Number => parse_f64(x)?,
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
