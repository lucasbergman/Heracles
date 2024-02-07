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
    response::Response,
    routing::get,
    Json, Router,
};
use axum_macros::debug_handler;
use maud::{html, Markup, PreEscaped};
use tracing::debug;

use crate::dashboard::{Dashboard, Graph};
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

// TODO(jwall): This should probably be encapsulated in a web component?
pub fn graph_component(dash_idx: usize, graph_idx: usize, graph: &Graph) -> Markup {
    let graph_id = format!("graph-{}-{}", dash_idx, graph_idx);
    // initialize the plot with Plotly.react
    // Update plot with Plotly.react which is more efficient
    let script = format!(
        "var data = []; Plotly.react('{}', data, {{ width: 500, height: 500 }});",
        graph_id
    );
    html!(
        div {
            h2 { (graph.title) }
            script {
                (script)
            }
            div id=(graph_id) { }
        }
    )
}

pub async fn graph_ui(
    State(config): State<Config>,
    Path((dash_idx, graph_idx)): Path<(usize, usize)>,
) -> Markup {
    let graph = config
        .get(dash_idx)
        .expect("No such dashboard")
        .graphs
        .get(graph_idx)
        .expect("No such graph");
    graph_component(dash_idx, graph_idx, graph)
}

pub async fn dash_ui(State(config): State<Config>, Path(dash_idx): Path<usize>) -> Markup {
    // TODO(zaphar): Should do better http error reporting here.
    let dash = config.get(dash_idx).expect("No such dashboard");
    let graph_iter = dash
        .graphs
        .iter()
        .enumerate()
        .collect::<Vec<(usize, &Graph)>>();
    html!(
        h1 { (dash.title) }
        @for (idx, graph) in &graph_iter {
            (graph_component(dash_idx, *idx, *graph))
        }
    )
}

pub fn mk_ui_routes(config: Arc<Vec<Dashboard>>) -> Router<Config> {
    Router::new()
        .route(
            "/dash/:dash_idx",
            get(dash_ui).with_state(State(config.clone())),
        )
        .route(
            "/dash/:dash_idx/graph/:graph_idx",
            get(graph_ui).with_state(State(config)),
        )
}

pub async fn index(State(config): State<Config>) -> Markup {
    html! {
        html {
            head {
                title { ("Heracles - Prometheus Unshackled") }
            }
            body {
                script src="/js/plotly.js" { }
                script src="/js/htmx.js" {  }
                (app(State(config.clone())).await)
            }
        }
    }
}

pub async fn app(State(config): State<Config>) -> Markup {
    let titles = config
        .iter()
        .map(|d| d.title.clone())
        .enumerate()
        .collect::<Vec<(usize, String)>>();
    html! {
        div {
            // Header menu
            ul {
                @for title in &titles {
                    li hx-get=(format!("/ui/dash/{}", title.0)) hx-target="#dashboard" { (title.1) }
                }
            }
            // dashboard display
            div id="dashboard" { }
        }
    }
}

pub fn javascript_response(content: &str) -> Response<String> {
    Response::builder()
        .header("Content-Type", "text/javascript")
        .body(content.to_string())
        .expect("Invalid javascript response")
}

pub async fn htmx() -> Response<String> {
    javascript_response(include_str!("../static/htmx.min.js"))
}

pub async fn plotly() -> Response<String> {
    javascript_response(include_str!("../static/plotly-2.27.0.min.js"))
}

pub fn mk_js_routes(config: Arc<Vec<Dashboard>>) -> Router<Config> {
    Router::new()
        .route("/plotly.js", get(plotly))
        .route("/htmx.js", get(htmx))
        .with_state(State(config))
}
