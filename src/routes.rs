// Copyright 2021 Jeremy Wall
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
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use maud::{html, Markup};
use tracing::debug;

use crate::dashboard::Dashboard;
use crate::query::{to_samples, QueryResult};

type Config = State<Arc<Vec<Dashboard>>>;

//#[axum_macros::debug_handler]
pub async fn graph_query(
    State(config): Config,
    Path((dash_idx, graph_idx)): Path<(usize, usize)>,
) -> Json<QueryResult> {
    debug!("Getting data for query");
    let graph = config
        .get(dash_idx)
        .expect("No such dashboard index")
        .graphs
        .get(graph_idx)
        .expect(&format!("No such graph in dasboard {}", dash_idx));
    let data = to_samples(
        graph
            .get_query_connection()
            .get_results()
            .await
            .expect("Unable to get query results")
            .data()
            .clone(),
    );
    Json(data)
}

pub fn mk_api_routes(config: Arc<Vec<Dashboard>>) -> Router<Config> {
    // Query routes
    Router::new().route(
        "/dash/:dash_idx/graph/:graph_idx",
        get(graph_query).with_state(config),
    )
}

pub fn mk_ui_routes(config: Arc<Vec<Dashboard>>) -> Router<Config> {
    Router::new()
}

pub async fn index(State(config): Config) -> Markup {
    html! {
        html {
            head {
                title { ("Heracles - Prometheus Unshackled") }
            }
            body {
                (app(State(config.clone())).await)
            }
        }
    }
}

pub async fn app(State(config): Config) -> Markup {
    let titles = config
        .iter()
        .map(|d| d.title.clone())
        .collect::<Vec<String>>();
    html! {
        div {
            // Header menu
            ul {
                @for title in &titles {
                    li { (title) }
                }
            }
            // dashboard display
            div { }
        }
    }
}
