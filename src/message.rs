use crate::{
    sections::{
        bitmap::BitmapSection,
        data::DataSection,
        data_representation::DataRepresentationSection,
        end::{self, EndSection},
        grid_definition::GridDefinitionSection,
        identification::IdentificationSection,
        indicator::{Discipline, IndicatorSection},
        product_definition::ProductDefinitionSection,
        section::Section,
    },
    templates::{data_representation::DataRepresentationTemplate, product::ProductTemplate},
};
use chrono::{DateTime, Utc};
use grib_types::Parameter;
use std::fmt::Display;
use std::vec::Vec;

pub struct MessageMetadata {
    pub discipline: Discipline,
    pub reference_date: DateTime<Utc>,
    pub forecast_date: DateTime<Utc>,
    pub variable_name: String,
    pub variable_abbreviation: String,
    pub region: ((f64, f64), (f64, f64)),
    pub location_grid: (usize, usize),
    pub location_resolution: (f64, f64),
    pub units: String,
    pub data_template_number: u16,
    pub data_point_count: usize,
}

pub struct Message<'a> {
    pub indicator_section: IndicatorSection<'a>,
    pub identification_section: IdentificationSection<'a>,
    pub grid_definition_section: GridDefinitionSection<'a>,
    pub product_definition_section: ProductDefinitionSection<'a>,
    pub data_representation_section: DataRepresentationSection<'a>,
    pub bitmap_section: BitmapSection<'a>,
    pub data_section: DataSection<'a>,
    pub end_section: EndSection<'a>,
}

impl<'a> Message<'a> {
    pub fn parse(data: &'a [u8], offset: usize) -> Result<Message<'a>, &'static str> {
        let mut indicator_section: Option<IndicatorSection<'a>> = None;
        let mut identification_section: Option<IdentificationSection<'a>> = None;
        let mut grid_definition_section: Option<GridDefinitionSection<'a>> = None;
        let mut product_definition_section: Option<ProductDefinitionSection<'a>> = None;
        let mut data_representation_section: Option<DataRepresentationSection<'a>> = None;
        let mut bitmap_section: Option<BitmapSection<'a>> = None;
        let mut data_section: Option<DataSection<'a>> = None;
        let mut end_section: Option<EndSection<'a>> = None;

        let mut current_offset = 0;
        let mut current_section = 0;
        loop {
            if current_section > 7 {
                break;
            }

            let next_section = Section::from_data(data, offset + current_offset)?;
            current_offset += next_section.len();

            match next_section {
                Section::Indicator(s) => indicator_section = Some(s),
                Section::Identification(s) => identification_section = Some(s),
                Section::LocalUse(s) => {}
                Section::GridDefinition(s) => grid_definition_section = Some(s),
                Section::ProductDefinition(s) => product_definition_section = Some(s),
                Section::DataRepresentation(s) => data_representation_section = Some(s),
                Section::Bitmap(s) => bitmap_section = Some(s),
                Section::Data(s) => data_section = Some(s),
                Section::End(s) => end_section = Some(s),
            }

            current_section += 1;
        }

        Ok(Message {
            indicator_section: indicator_section.unwrap(),
            identification_section: identification_section.unwrap(),
            grid_definition_section: grid_definition_section.unwrap(),
            product_definition_section: product_definition_section.unwrap(),
            data_representation_section: data_representation_section.unwrap(),
            bitmap_section: bitmap_section.unwrap(),
            data_section: data_section.unwrap(),
            end_section: end_section.unwrap(),
        })
    }

    pub fn parse_all(data: &'a [u8]) -> Vec<Message<'a>> {
        let mut messages = Vec::new();
        let mut offset: usize = 0;

        while offset < data.len() {
            if let Ok(message) = Message::parse(data, offset) {
                offset += message.len();
                messages.push(message);
            } else {
                break;
            }
        }

        messages
    }

    pub fn variable_names(messages: Vec<Message<'a>>) -> Vec<Option<String>> {
        Message::parameters(messages)
            .iter()
            .map(|p| match p {
                Some(p) => Some(p.name.clone()),
                None => None,
            })
            .collect()
    }

    pub fn variable_abbrevs(messages: Vec<Message<'a>>) -> Vec<Option<String>> {
        Message::parameters(messages)
            .iter()
            .map(|p| match p {
                Some(p) => Some(p.abbrev.clone()),
                None => None,
            })
            .collect()
    }

    pub fn units(messages: Vec<Message<'a>>) -> Vec<Option<String>> {
        Message::parameters(messages)
            .iter()
            .map(|p| match p {
                Some(p) => Some(p.unit.clone()),
                None => None,
            })
            .collect()
    }

    pub fn parameters(messages: Vec<Message<'a>>) -> Vec<Option<Parameter>> {
        messages
            .iter()
            .map(|m| m.parameter())
            .map(|r| match r {
                Ok(parameter) => Some(parameter),
                Err(_) => None,
            })
            .collect()
    }

    pub fn forecast_dates(messages: Vec<Message<'a>>) -> Vec<Option<DateTime<Utc>>> {
        messages
            .iter()
            .map(|m| m.forecast_date())
            .map(|r| match r {
                Ok(date) => Some(date),
                Err(_) => None,
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.indicator_section.total_length() as usize
    }

    // pub fn section_count(&self) -> usize {
    //     self.sections.len()
    // }

    pub fn discipline(&self) -> Discipline {
        self.indicator_section.discipline()
    }

    pub fn parameter(&self) -> Result<Parameter, String> {
        let discipline = self.discipline();

        let product_template = unwrap_or_return!(
            match self
                .product_definition_section
                .product_definition_template(discipline.clone() as u8)
            {
                ProductTemplate::HorizontalAnalysisForecast(template) => Some(template),
                _ => None,
            },
            "Only HorizontalAnalysisForecast templates are supported at this time".into()
        );

        let parameter = unwrap_or_return!(
            product_template.parameter(),
            "This Product and Parameter is currently not supported".into()
        );

        Ok(parameter)
    }

    pub fn variable_name(&self) -> Result<String, String> {
        let parameter = self.parameter()?;
        Ok(parameter.name)
    }

    pub fn variable_abbrev(&self) -> Result<String, String> {
        let parameter = self.parameter()?;
        Ok(parameter.abbrev)
    }

    pub fn reference_date(&self) -> DateTime<Utc> {
        self.identification_section.reference_date()
    }

    pub fn forecast_date(&self) -> Result<DateTime<Utc>, String> {
        let discipline = self.discipline();

        let product_template = unwrap_or_return!(
            match self
                .product_definition_section
                .product_definition_template(discipline.clone() as u8)
            {
                ProductTemplate::HorizontalAnalysisForecast(template) => Some(template),
                _ => None,
            },
            "Only HorizontalAnalysisForecast templates are supported at this time".into()
        );

        let reference_date = self.reference_date();
        Ok(product_template.forecast_datetime(reference_date))
    }

    pub fn metadata(&self) -> Result<MessageMetadata, String> {
        let discipline = self.discipline();

        let reference_date = self.reference_date();

        let grid_template = unwrap_or_return!(
            self.grid_definition_section.grid_definition_template(),
            "Only latitude longitude templates supported at this time".into()
        );
        let region = (grid_template.start(), grid_template.end());
        let location_grid = (
            grid_template.latitude_count(),
            grid_template.longitude_count(),
        );
        let location_resolution = (
            grid_template.latitude_resolution(),
            grid_template.longitude_resolution(),
        );

        let parameter = self.parameter()?;

        let forecast_date = self.forecast_date()?;

        let data_template_number = self
            .data_representation_section
            .data_representation_template_number();
        let data_point_count = self.grid_definition_section.data_point_count();

        Ok(MessageMetadata {
            discipline,
            reference_date,
            forecast_date,
            variable_name: parameter.name,
            variable_abbreviation: parameter.abbrev,
            region,
            location_grid,
            location_resolution,
            units: parameter.unit,
            data_template_number,
            data_point_count,
        })
    }

    pub fn data(&self) -> Result<Vec<f64>, String> {
        let raw_packed_data = self.data_section.raw_bit_data();
        println!("data sectionln: {}", raw_packed_data.len());

        let data_representation_template = unwrap_or_return!(
            self.data_representation_section
                .data_representation_template(),
            "Failed to unpack the data representation template".into()
        );

        let scaled_unpacked_data = data_representation_template.unpack_all(raw_packed_data)?;

        let mapped_scaled_data = self.bitmap_section.map_data(scaled_unpacked_data);
        Ok(mapped_scaled_data)
    }

    pub fn data_locations(&self) -> Result<Vec<(f64, f64)>, String> {
        let grid_template = unwrap_or_return!(
            self.grid_definition_section.grid_definition_template(),
            "Only latitude longitude templates supported at this time".into()
        );

        Ok(grid_template.locations())
    }

    pub fn data_at_location(&self, location: &(f64, f64)) -> Result<f64, String> {
        let grid_template = unwrap_or_return!(
            self.grid_definition_section.grid_definition_template(),
            "Only latitude longitude templates supported at this time".into()
        );

        let location_index = grid_template.index_for_location(location.0, location.1)?;

        let data_representation_template = unwrap_or_return!(
            self.data_representation_section
                .data_representation_template(),
            "Failed to unpack the data representation template".into()
        );

        let data_index = unwrap_or_return!(
            self.bitmap_section.data_index(location_index),
            format!("No data available at index {}", location_index).into()
        );

        let raw_packed_data = self.data_section.raw_bit_data();
        let data = data_representation_template
            .unpack_range(raw_packed_data, data_index..data_index + 1)?;

        Ok(data[0])
    }
}
