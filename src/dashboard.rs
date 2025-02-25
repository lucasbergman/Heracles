// Copyright 2023 Jeremy Wall
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use std::path::Path;

use chrono::prelude::*;
use chrono::Duration;
use serde::Deserialize;
use serde_yaml;
use tracing::{debug, error};

use crate::query::{QueryConn, QueryType, PlotMeta};

#[derive(Deserialize, Debug)]
pub struct GraphSpan {
    // serialized with https://datatracker.ietf.org/doc/html/rfc3339 and special handling for 'now'
    pub end: String,
    pub duration: String,
    pub step_duration: String,
}

#[derive(Deserialize)]
pub struct Dashboard {
    pub title: String,
    pub graphs: Vec<Graph>,
    pub span: Option<GraphSpan>,
}

#[derive(Deserialize)]
pub struct SubPlot {
    pub source: String,
    pub query: String,
    pub meta: PlotMeta,
}

#[derive(Deserialize)]
pub struct Graph {
    pub title: String,
    pub plots: Vec<SubPlot>,
    pub span: Option<GraphSpan>,
    pub query_type: QueryType,
    pub d3_tick_format: Option<String>,
}

fn duration_from_string(duration_string: &str) -> Option<Duration> {
    match parse_duration::parse(duration_string) {
        Ok(d) => match Duration::from_std(d) {
            Ok(d) => Some(d),
            Err(e) => {
                error!(err = ?e, "specified Duration is out of bounds");
                return None;
            }
        },
        Err(e) => {
            error!(
                err = ?e,
                "Failed to parse duration"
            );
            return None;
        }
    }
}

fn graph_span_to_tuple(span: &Option<GraphSpan>) -> Option<(DateTime<Utc>, Duration, Duration)> {
    if span.is_none() {
        return None;
    }
    let span = span.as_ref().unwrap();
    let duration = match duration_from_string(&span.duration) {
        Some(d) => d,
        None => {
            error!("Invalid query duration not assigning span to to graph query");
            return None;
        }
    };
    let step_duration = match duration_from_string(&span.step_duration) {
        Some(d) => d,
        None => {
            error!("Invalid query step resolution not assigning span to to graph query");
            return None;
        }
    };
    let end = if span.end == "now" {
        Utc::now()
    } else if let Ok(end) = DateTime::parse_from_rfc3339(&span.end) {
        end.to_utc()
    } else {
        error!(?span.end, "Invalid DateTime using current time.");
        Utc::now()
    };
    Some((end, duration, step_duration))
}

impl Graph {
    pub fn get_query_connections<'conn, 'graph: 'conn>(
        &'graph self,
        graph_span: &'graph Option<GraphSpan>,
        query_span: &'graph Option<GraphSpan>,
    ) -> Vec<QueryConn<'conn>> {
        let mut conns = Vec::new();
        for plot in self.plots.iter() {
            debug!(
                query = plot.query,
                source = plot.source,
                "Getting query connection for graph"
            );
            let mut conn = QueryConn::new(&plot.source, &plot.query, self.query_type.clone(), plot.meta.clone());
            // Query params take precendence over all other settings. Then graph settings take
            // precedences and finally the dashboard settings take precendence
            if let Some((end, duration, step_duration)) = graph_span_to_tuple(query_span) {
                conn = conn.with_span(end, duration, step_duration);
            } else if let Some((end, duration, step_duration)) = graph_span_to_tuple(&self.span) {
                conn = conn.with_span(end, duration, step_duration);
            } else if let Some((end, duration, step_duration)) = graph_span_to_tuple(graph_span) {
                conn = conn.with_span(end, duration, step_duration);
            }
            conns.push(conn);
        }
        conns
    }
}

pub fn read_dashboard_list(path: &Path) -> anyhow::Result<Vec<Dashboard>> {
    let f = std::fs::File::open(path)?;
    Ok(serde_yaml::from_reader(f)?)
}
