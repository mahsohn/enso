//! The module with the [`Graph`] presenter. See [`crate::presenter`] documentation to know more
//! about presenters in general.

pub mod call_stack;
pub mod state;
pub mod visualization;

pub use call_stack::CallStack;
pub use visualization::Visualization;

use crate::prelude::*;

use crate::controller::upload::NodeFromDroppedFileHandler;
use crate::executor::global::spawn_stream_handler;
use crate::presenter::graph::state::State;

use enso_frp as frp;
use futures::future::LocalBoxFuture;
use ide_view as view;
use ide_view::graph_editor::component::node as node_view;
use ide_view::graph_editor::component::visualization as visualization_view;
use ide_view::graph_editor::EdgeEndpoint;


// ===============
// === Aliases ===
// ===============

/// The node identifier used by view.
pub type ViewNodeId = view::graph_editor::NodeId;

/// The node identifier used by controllers.
pub type AstNodeId = ast::Id;

/// The connection identifier used by view.
pub type ViewConnection = view::graph_editor::EdgeId;

/// The connection identifier used by controllers.
pub type AstConnection = controller::graph::Connection;



// =================
// === Constants ===
// =================

/// The identifier base that will be used to name the methods introduced by "collapse nodes"
/// refactoring. Names are typically generated by taking base and appending subsequent integers,
/// until the generated name does not collide with any known identifier.
const COLLAPSED_FUNCTION_NAME: &str = "func";

/// The default X position of the node when user did not set any position of node - possibly when
/// node was added by editing text.
const DEFAULT_NODE_X_POSITION: f32 = -100.0;
/// The default Y position of the node when user did not set any position of node - possibly when
/// node was added by editing text.
const DEFAULT_NODE_Y_POSITION: f32 = 200.0;

/// Default node position -- acts as a starting points for laying out nodes with no position defined
/// in the metadata.
pub fn default_node_position() -> Vector2 {
    Vector2::new(DEFAULT_NODE_X_POSITION, DEFAULT_NODE_Y_POSITION)
}



// =============
// === Model ===
// =============

#[derive(Debug)]
struct Model {
    logger:           Logger,
    project:          model::Project,
    controller:       controller::ExecutedGraph,
    view:             view::graph_editor::GraphEditor,
    state:            Rc<State>,
    _visualization:   Visualization,
    _execution_stack: CallStack,
}

impl Model {
    pub fn new(
        project: model::Project,
        controller: controller::ExecutedGraph,
        view: view::graph_editor::GraphEditor,
    ) -> Self {
        let logger = Logger::new("presenter::Graph");
        let state: Rc<State> = default();
        let visualization = Visualization::new(
            project.clone_ref(),
            controller.clone_ref(),
            view.clone_ref(),
            state.clone_ref(),
        );
        let execution_stack =
            CallStack::new(&logger, controller.clone_ref(), view.clone_ref(), state.clone_ref());
        Self {
            logger,
            project,
            controller,
            view,
            state,
            _visualization: visualization,
            _execution_stack: execution_stack,
        }
    }

    /// Node position was changed in view.
    fn node_position_changed(&self, id: ViewNodeId, position: Vector2) {
        self.update_ast(
            || {
                let ast_id = self.state.update_from_view().set_node_position(id, position)?;
                Some(self.controller.graph().set_node_position(ast_id, position))
            },
            "update node position",
        );
    }

    fn node_visualization_changed(&self, id: ViewNodeId, path: Option<visualization_view::Path>) {
        self.update_ast(
            || {
                let ast_id =
                    self.state.update_from_view().set_node_visualization(id, path.clone())?;
                let module = self.controller.graph().module;
                let result = match serde_json::to_value(path) {
                    Ok(serialized) => module
                        .with_node_metadata(ast_id, Box::new(|md| md.visualization = serialized)),
                    Err(err) => FallibleResult::Err(err.into()),
                };
                Some(result)
            },
            "update node position",
        );
    }

    /// Node was removed in view.
    fn node_removed(&self, id: ViewNodeId) {
        self.update_ast(
            || {
                let ast_id = self.state.update_from_view().remove_node(id)?;
                Some(self.controller.graph().remove_node(ast_id))
            },
            "remove node",
        )
    }

    /// Connection was created in view.
    fn new_connection_created(&self, id: ViewConnection) {
        self.update_ast(
            || {
                let connection = self.view.model.edges.get_cloned_ref(&id)?;
                let ast_to_create = self.state.update_from_view().create_connection(connection)?;
                Some(self.controller.connect(&ast_to_create))
            },
            "create connection",
        );
    }

    /// Connection was removed in view.
    fn connection_removed(&self, id: ViewConnection) {
        self.update_ast(
            || {
                let ast_to_remove = self.state.update_from_view().remove_connection(id)?;
                Some(self.controller.disconnect(&ast_to_remove))
            },
            "delete connection",
        );
    }

    fn nodes_collapsed(&self, collapsed: &[ViewNodeId]) {
        self.update_ast(
            || {
                debug!(self.logger, "Collapsing node.");
                let ids = collapsed.iter().filter_map(|node| self.state.ast_node_id_of_view(*node));
                let new_node_id = self.controller.graph().collapse(ids, COLLAPSED_FUNCTION_NAME);
                // TODO [mwu] https://github.com/enso-org/ide/issues/760
                //   As part of this issue, storing relation between new node's controller and view
                //   ids will be necessary.
                Some(new_node_id.map(|_| ()))
            },
            "collapse nodes",
        );
    }

    fn update_ast<F>(&self, f: F, action: &str)
    where F: FnOnce() -> Option<FallibleResult> {
        if let Some(Err(err)) = f() {
            error!(self.logger, "Failed to {action} in AST: {err}");
        }
    }

    /// Extract all types for subexpressions in node expressions, update the state,
    /// and return the events for graph editor FRP input setting all of those types.
    ///
    /// The result includes the types not changed according to the state. That's because this
    /// function is used after node expression change, and we need to reset all the types in view.
    fn all_types_of_node(
        &self,
        node: ViewNodeId,
    ) -> Vec<(ViewNodeId, ast::Id, Option<view::graph_editor::Type>)> {
        let subexpressions = self.state.expressions_of_node(node);
        subexpressions
            .iter()
            .map(|id| {
                let a_type = self.expression_type(*id);
                self.state.update_from_controller().set_expression_type(*id, a_type.clone());
                (node, *id, a_type)
            })
            .collect()
    }

    /// Extract all method pointers for subexpressions, update the state, and return events updating
    /// view for expressions where method pointer actually changed.
    fn all_method_pointers_of_node(
        &self,
        node: ViewNodeId,
    ) -> Vec<(ast::Id, Option<view::graph_editor::MethodPointer>)> {
        let subexpressions = self.state.expressions_of_node(node);
        subexpressions.iter().filter_map(|id| self.refresh_expression_method_pointer(*id)).collect()
    }

    /// Refresh type of the given expression.
    ///
    /// If the view update is required, the GraphEditor's FRP input event is returned.
    fn refresh_expression_type(
        &self,
        id: ast::Id,
    ) -> Option<(ViewNodeId, ast::Id, Option<view::graph_editor::Type>)> {
        let a_type = self.expression_type(id);
        let node_view =
            self.state.update_from_controller().set_expression_type(id, a_type.clone())?;
        Some((node_view, id, a_type))
    }

    /// Refresh method pointer of the given expression.
    ///
    /// If the view update is required, the GraphEditor's FRP input event is returned.
    fn refresh_expression_method_pointer(
        &self,
        id: ast::Id,
    ) -> Option<(ast::Id, Option<view::graph_editor::MethodPointer>)> {
        let method_pointer = self.expression_method(id);
        self.state
            .update_from_controller()
            .set_expression_method_pointer(id, method_pointer.clone())?;
        Some((id, method_pointer))
    }

    fn refresh_node_error(
        &self,
        expression: ast::Id,
    ) -> Option<(ViewNodeId, Option<node_view::Error>)> {
        let registry = self.controller.computed_value_info_registry();
        let payload = registry.get(&expression).map(|info| info.payload.clone());
        self.state.update_from_controller().set_node_error_from_payload(expression, payload)
    }

    /// Extract the expression's current type from controllers.
    fn expression_type(&self, id: ast::Id) -> Option<view::graph_editor::Type> {
        let registry = self.controller.computed_value_info_registry();
        let info = registry.get(&id)?;
        Some(view::graph_editor::Type(info.typename.as_ref()?.clone_ref()))
    }

    /// Extract the expression's current method pointer from controllers.
    fn expression_method(&self, id: ast::Id) -> Option<view::graph_editor::MethodPointer> {
        let registry = self.controller.computed_value_info_registry();
        let method_id = registry.get(&id)?.method_call?;
        let suggestion_db = self.controller.graph().suggestion_db.clone_ref();
        let method = suggestion_db.lookup_method_ptr(method_id).ok()?;
        Some(view::graph_editor::MethodPointer(Rc::new(method)))
    }

    fn file_dropped(&self, file: ensogl_drop_manager::File, position: Vector2<f32>) {
        let project = self.project.clone_ref();
        let graph = self.controller.graph();
        let to_upload = controller::upload::FileToUpload {
            name: file.name.clone_ref().into(),
            size: file.size,
            data: file,
        };
        let position = model::module::Position { vector: position };
        let handler = NodeFromDroppedFileHandler::new(&self.logger, project, graph);
        if let Err(err) = handler.create_node_and_start_uploading(to_upload, position) {
            error!(self.logger, "Error when creating node from dropped file: {err}");
        }
    }

    /// Look through all graph's nodes in AST and set position where it is missing.
    fn initialize_nodes_positions(&self, default_gap_between_nodes: f32) {
        match self.controller.graph().nodes() {
            Ok(nodes) => {
                use model::module::Position;

                let base_default_position = default_node_position();
                let node_positions =
                    nodes.iter().filter_map(|node| node.metadata.as_ref()?.position);
                let bottommost_pos = node_positions
                    .min_by(Position::ord_by_y)
                    .map(|p| p.vector)
                    .unwrap_or(base_default_position);

                let offset = default_gap_between_nodes + node_view::HEIGHT;
                let mut next_default_position =
                    Vector2::new(bottommost_pos.x, bottommost_pos.y - offset);

                let transaction =
                    self.controller.get_or_open_transaction("Setting default positions.");
                transaction.ignore();
                for node in nodes {
                    if !node.has_position() {
                        if let Err(err) = self
                            .controller
                            .graph()
                            .set_node_position(node.id(), next_default_position)
                        {
                            warning!(
                                self.logger,
                                "Failed to initialize position of node {node.id()}: {err}"
                            );
                        }
                        next_default_position.y -= offset;
                    }
                }
            }
            Err(err) => {
                warning!(self.logger, "Failed to initialize nodes positions: {err}");
            }
        }
    }
}



// ==================
// === ViewUpdate ===
// ==================

/// Structure handling view update after graph invalidation.
///
/// Because updating various graph elements (nodes, connections, types) bases on the same data
/// extracted from controllers, the data are cached in this structure.
#[derive(Clone, Debug, Default)]
struct ViewUpdate {
    state:       Rc<State>,
    nodes:       Vec<controller::graph::Node>,
    trees:       HashMap<AstNodeId, controller::graph::NodeTrees>,
    connections: HashSet<AstConnection>,
}

impl ViewUpdate {
    /// Create ViewUpdate information from Graph Presenter's model.
    fn new(model: &Model) -> FallibleResult<Self> {
        let state = model.state.clone_ref();
        let nodes = model.controller.graph().nodes()?;
        let connections_and_trees = model.controller.connections()?;
        let connections = connections_and_trees.connections.into_iter().collect();
        let trees = connections_and_trees.trees;
        Ok(Self { state, nodes, trees, connections })
    }

    /// Remove nodes from the state and return node views to be removed.
    fn remove_nodes(&self) -> Vec<ViewNodeId> {
        self.state.update_from_controller().retain_nodes(&self.node_ids().collect())
    }

    /// Returns number of nodes view should create.
    fn count_nodes_to_add(&self) -> usize {
        self.node_ids().filter(|n| self.state.view_id_of_ast_node(*n).is_none()).count()
    }

    /// Set the nodes expressions in state, and return the events to be passed to Graph Editor FRP
    /// input for nodes where expression changed.
    ///
    /// The nodes not having views are also updated in the state.
    fn set_node_expressions(&self) -> Vec<(ViewNodeId, node_view::Expression)> {
        self.nodes
            .iter()
            .filter_map(|node| {
                let id = node.main_line.id();
                let trees = self.trees.get(&id).cloned().unwrap_or_default();
                self.state.update_from_controller().set_node_expression(node, trees)
            })
            .collect()
    }

    /// Set the nodes position in state, and return the events to be passed to GraphEditor FRP
    /// input for nodes where position changed.
    ///
    /// The nodes not having views are also updated in the state.
    fn set_node_positions(&self) -> Vec<(ViewNodeId, Vector2)> {
        self.nodes
            .iter()
            .filter_map(|node| {
                let id = node.main_line.id();
                let position = node.position().map(|p| p.vector)?;
                let view_id =
                    self.state.update_from_controller().set_node_position(id, position)?;
                Some((view_id, position))
            })
            .collect()
    }

    fn set_node_visualizations(&self) -> Vec<(ViewNodeId, Option<visualization_view::Path>)> {
        self.nodes
            .iter()
            .filter_map(|node| {
                let data = node.metadata.as_ref().map(|md| md.visualization.clone());
                self.state.update_from_controller().set_node_visualization(node.id(), data)
            })
            .collect()
    }

    /// Remove connections from the state and return views to be removed.
    fn remove_connections(&self) -> Vec<ViewConnection> {
        self.state.update_from_controller().retain_connections(&self.connections)
    }

    /// Add connections to the state and return endpoints of connections to be created in views.
    fn add_connections(&self) -> Vec<(EdgeEndpoint, EdgeEndpoint)> {
        let ast_conns = self.connections.iter();
        ast_conns
            .filter_map(|connection| {
                self.state.update_from_controller().set_connection(connection.clone())
            })
            .collect()
    }

    fn node_ids(&self) -> impl Iterator<Item = AstNodeId> + '_ {
        self.nodes.iter().map(controller::graph::Node::id)
    }
}



// =============
// === Graph ===
// =============

/// The Graph Presenter, synchronizing graph state between graph controller and view.
///
/// This presenter focuses on the graph structure: nodes, their expressions and types, and
/// connections between them. It does not integrate Searcher nor Breadcrumbs (managed by
/// [`presenter::Searcher`] and [`presenter::CallStack`] respectively).
#[derive(Debug)]
pub struct Graph {
    network: frp::Network,
    model:   Rc<Model>,
}

impl Graph {
    /// Create graph presenter. The returned structure is working and does not require any
    /// initialization.
    pub fn new(
        project: model::Project,
        controller: controller::ExecutedGraph,
        project_view: &view::project::View,
    ) -> Self {
        let network = frp::Network::new("presenter::Graph");
        let view = project_view.graph().clone_ref();
        let model = Rc::new(Model::new(project, controller, view));
        Self { network, model }.init(project_view)
    }

    fn init(self, project_view: &view::project::View) -> Self {
        let logger = &self.model.logger;
        let network = &self.network;
        let model = &self.model;
        let view = &self.model.view.frp;
        frp::extend! { network
            update_view <- source::<()>();
            // Position initialization should go before emitting `update_data` event.
            update_with_gap <- view.default_y_gap_between_nodes.sample(&update_view);
            eval update_with_gap ((gap) model.initialize_nodes_positions(*gap));
            update_data <- update_view.map(f_!([logger,model] match ViewUpdate::new(&*model) {
                Ok(update) => Rc::new(update),
                Err(err) => {
                    error!(logger,"Failed to update view: {err:?}");
                    Rc::new(default())
                }
            }));


            // === Refreshing Nodes ===

            remove_node <= update_data.map(|update| update.remove_nodes());
            update_node_expression <= update_data.map(|update| update.set_node_expressions());
            set_node_position <= update_data.map(|update| update.set_node_positions());
            set_node_visualization <= update_data.map(|update| update.set_node_visualizations());
            enable_vis <- set_node_visualization.filter_map(|(id,path)| path.is_some().as_some(*id));
            disable_vis <- set_node_visualization.filter_map(|(id,path)| path.is_none().as_some(*id));
            view.remove_node <+ remove_node;
            view.set_node_expression <+ update_node_expression;
            view.set_node_position <+ set_node_position;
            view.set_visualization <+ set_node_visualization;
            view.enable_visualization <+ enable_vis;
            view.disable_visualization <+ disable_vis;

            view.add_node <+ update_data.map(|update| update.count_nodes_to_add()).repeat();
            added_node_update <- view.node_added.filter_map(f!(((view_id,_))
                model.state.assign_node_view(*view_id)
            ));
            init_node_expression <- added_node_update.filter_map(|update| Some((update.view_id?, update.expression.clone())));
            view.set_node_expression <+ init_node_expression;
            view.set_node_position <+ added_node_update.filter_map(|update| Some((update.view_id?, update.position)));
            view.set_visualization <+ added_node_update.filter_map(|update| Some((update.view_id?, Some(update.visualization.clone()?))));
            view.enable_visualization <+ added_node_update.filter_map(|update| update.visualization.is_some().and_option(update.view_id));


            // === Refreshing Connections ===

            remove_connection <= update_data.map(|update| update.remove_connections());
            add_connection <= update_data.map(|update| update.add_connections());
            view.remove_edge <+ remove_connection;
            view.connect_nodes <+ add_connection;


            // === Refreshing Expressions ===

            reset_node_types <- any(update_node_expression, init_node_expression)._0();
            set_expression_type <= reset_node_types.map(f!((view_id) model.all_types_of_node(*view_id)));
            set_method_pointer <= reset_node_types.map(f!((view_id) model.all_method_pointers_of_node(*view_id)));
            view.set_expression_usage_type <+ set_expression_type;
            view.set_method_pointer <+ set_method_pointer;

            update_expressions <- source::<Vec<ast::Id>>();
            update_expression <= update_expressions;
            view.set_expression_usage_type <+ update_expression.filter_map(f!((id) model.refresh_expression_type(*id)));
            view.set_method_pointer <+ update_expression.filter_map(f!((id) model.refresh_expression_method_pointer(*id)));
            view.set_node_error_status <+ update_expression.filter_map(f!((id) model.refresh_node_error(*id)));


            // === Changes from the View ===

            eval view.node_position_set_batched(((node_id, position)) model.node_position_changed(*node_id, *position));
            eval view.node_removed((node_id) model.node_removed(*node_id));
            eval view.on_edge_endpoints_set((edge_id) model.new_connection_created(*edge_id));
            eval view.on_edge_endpoint_unset(((edge_id,_)) model.connection_removed(*edge_id));
            eval view.nodes_collapsed(((nodes, _)) model.nodes_collapsed(nodes));
            eval view.enabled_visualization_path(((node_id, path)) model.node_visualization_changed(*node_id, path.clone()));


            // === Dropping Files ===

            file_upload_requested <- view.file_dropped.gate(&project_view.drop_files_enabled);
            eval file_upload_requested (((file,position)) model.file_dropped(file.clone_ref(),*position));
        }

        view.remove_all_nodes();
        update_view.emit(());
        self.setup_controller_notification_handlers(update_view, update_expressions);

        self
    }

    fn setup_controller_notification_handlers(
        &self,
        update_view: frp::Source<()>,
        update_expressions: frp::Source<Vec<ast::Id>>,
    ) {
        use crate::controller::graph::executed;
        use crate::controller::graph::Notification;
        let graph_notifications = self.model.controller.subscribe();
        let weak = Rc::downgrade(&self.model);
        spawn_stream_handler(weak, graph_notifications, move |notification, model| {
            info!(model.logger, "Received controller notification {notification:?}");
            match notification {
                executed::Notification::Graph(graph) => match graph {
                    Notification::Invalidate => update_view.emit(()),
                    Notification::PortsUpdate => update_view.emit(()),
                },
                executed::Notification::ComputedValueInfo(expressions) =>
                    update_expressions.emit(expressions),
                executed::Notification::EnteredNode(_) => update_view.emit(()),
                executed::Notification::SteppedOutOfNode(_) => update_view.emit(()),
            }
            std::future::ready(())
        })
    }
}


// === State Access ===

impl Graph {
    /// Get the view id of given AST node.
    pub fn view_id_of_ast_node(&self, id: AstNodeId) -> Option<ViewNodeId> {
        self.model.state.view_id_of_ast_node(id)
    }

    /// Get the ast id of given node view.
    pub fn ast_node_of_view(&self, id: ViewNodeId) -> Option<AstNodeId> {
        self.model.state.ast_node_id_of_view(id)
    }

    /// Assign a node view to the given AST id. Since next update, the presenter will share the
    /// node content between the controllers and the view.
    pub fn assign_node_view_explicitly(&self, view_id: ViewNodeId, ast_id: AstNodeId) {
        self.model.state.assign_node_view_explicitly(view_id, ast_id);
    }
}



// ====================================
// === DataProvider for EnsoGL File ===
// ====================================

impl controller::upload::DataProvider for ensogl_drop_manager::File {
    fn next_chunk(&mut self) -> LocalBoxFuture<FallibleResult<Option<Vec<u8>>>> {
        self.read_chunk().map(|f| f.map_err(|e| e.into())).boxed_local()
    }
}
