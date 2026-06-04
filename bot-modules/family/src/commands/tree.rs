use charming::series::{GraphData, GraphLink, GraphNode, GraphNodeLabel};
use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
};
use sqlx::{Database, Pool};

use crate::Result;
use crate::family_manager::FamilyManager;

#[derive(Debug)]
struct Node {
    pub id: i64,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub value: f64,
    pub category: u64,
    pub symbol_size: f64,
    pub link: Vec<i64>,
}

impl Node {
    const fn new(id: i64, name: String, x: f64, y: f64) -> Self {
        Self {
            id,
            name,
            x,
            y,
            value: 0.0,
            category: 0,
            symbol_size: 100.0,
            link: Vec::new(),
        }
    }

    fn add_link(mut self, id: i64) -> Self {
        self.link.push(id);
        self
    }
}

impl From<&Node> for GraphNode {
    fn from(node: &Node) -> Self {
        Self {
            id: node.id.to_string(),
            name: node.name.clone(),
            x: node.x,
            y: node.y,
            value: node.value,
            category: node.category,
            symbol_size: node.symbol_size,
            label: Some(
                GraphNodeLabel::new()
                    .show(true)
                    .position("inside")
                    .formatter("{b}")
                    .color("white")
                    .font_size(22),
            ),
        }
    }
}

pub struct Tree;

impl Tree {
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss is acceptable for tree layout positioning"
    )]
    pub async fn run<Db: Database, Manager: FamilyManager<Db>>(
        _ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<GraphData> {
        let row = Manager::row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| (&interaction.user).into());

        let tree = row.tree::<Db, Manager>(pool).await?;

        let mut keys: Vec<i32> = tree.keys().copied().collect();
        keys.sort_unstable();

        let max_width = tree.values().map(Vec::len).max().unwrap_or(0);

        let mut nodes = Vec::new();
        for depth in keys {
            let values =
                tree.get(&depth).expect("key from tree.keys() always present");
            let width = values.len();
            let width_diff = max_width - width;
            let spacing = width_diff as f64 / 2.0;
            for (index, value) in values.iter().enumerate() {
                let mut node = Node::new(
                    value.id,
                    value.username.clone(),
                    spacing + index as f64,
                    f64::from(depth),
                );
                for id in value.children_ids.iter().chain(value.partner_ids.iter()) {
                    node = node.add_link(*id);
                }
                nodes.push(node);
            }
        }

        let data = GraphData {
            nodes: nodes.iter().map(GraphNode::from).collect(),
            links: nodes
                .iter()
                .flat_map(|node| {
                    node.link.iter().map(|link| GraphLink {
                        source: node.id.to_string(),
                        target: link.to_string(),
                        value: None,
                    })
                })
                .collect(),
            categories: Vec::new(),
        };

        Ok(data)
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("tree")
            .description("Display your family tree.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user whose family tree to display.",
            ))
    }
}
