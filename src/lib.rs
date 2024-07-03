//! crate doc comment here.
//!

mod tests;

use std::collections::HashMap;

use markdown::mdast::{self, Break, LinkReference, Node, ReferenceKind};
use markdown::{to_mdast, ParseOptions};
use notion_client::objects::emoji::Emoji;
use notion_client::objects::rich_text::{Annotations, Equation, Link, RichText, Text};
use notion_client::objects::{block::*, rich_text};

/// Convert a string slice containing Markdown into a vector of Notion document blocks.
pub fn convert(input: &str) -> Vec<Block> {
    // This function is infallible with the default options.
    let Ok(tree) = to_mdast(input, &ParseOptions::gfm()) else {
        return Vec::new();
    };
    let mut state = State::new();
    state.render(tree)
}

enum ListVariation {
    None,
    Bulleted,
    Ordered,
}

// first take on this
struct State {
    list: ListVariation,
    ordered_start: u32,
    links: HashMap<String, String>,
    footnotes: HashMap<String, String>,
}

impl State {
    pub fn new() -> Self {
        State {
            list: ListVariation::None,
            ordered_start: 1,
            links: HashMap::new(),
            footnotes: HashMap::new(),
        }
    }

    pub fn render(&mut self, tree: Node) -> Vec<Block> {
        if let Some(children) = tree.children() {
            self.render_nodes(children)
        } else {
            Vec::new()
        }
    }

    /// Render the passed-in vector of nodes.
    fn render_nodes(&mut self, nodelist: &[Node]) -> Vec<Block> {
        self.links = self.collect_definitions(nodelist);
        nodelist
            .iter()
            .map(|xs| self.render_node(xs))
            .flatten()
            .collect::<Vec<Block>>()
    }

    /// Collect link definitions.
    fn collect_definitions(&self, nodelist: &[Node]) -> HashMap<String, String> {
        nodelist
            .iter()
            .filter_map(|xs| {
                let Node::Definition(definition) = xs else {
                    return None;
                };
                Some((definition.identifier.clone(), definition.url.clone()))
            })
            .collect()
    }

    /// Render a node that becomes a Notion block.
    pub fn render_node(&mut self, node: &Node) -> Vec<Block> {
        match node {
            // Node::Root(_) => todo!(),
            Node::BlockQuote(quote) => vec![self.render_quote(quote)],
            Node::FootnoteDefinition(footnote) => vec![self.render_footnote(footnote)],
            Node::List(list) => self.begin_list(list),
            Node::FootnoteReference(reference) => vec![self.render_noteref(reference)],
            Node::Html(html) => vec![self.render_html(html)],
            Node::Image(image) => vec![self.render_image(image)],
            Node::ImageReference(_imgref) => todo!(),
            Node::Code(code) => vec![self.render_code(code)],
            Node::Math(math) => vec![self.render_math(math)],
            Node::Heading(heading) => vec![self.render_heading(heading)],
            Node::Table(_) => todo!(),
            Node::ThematicBreak(div) => vec![self.render_divider(div)],
            Node::TableRow(_) => todo!(),
            Node::TableCell(_) => todo!(),
            Node::ListItem(list_item) => vec![self.render_list_item(list_item)],
            Node::Paragraph(paragraph) => vec![self.render_paragraph(paragraph)],
            // All unhandled node types are deliberately skipped.
            _ => Vec::new(),
        }
    }

    /// Render a node type that becomes Notion rich text.
    fn render_text_node(&mut self, node: &Node) -> RichText {
        match node {
            Node::InlineCode(inline) => self.render_inline_code(inline),
            Node::InlineMath(math) => self.render_inline_math(math),
            Node::Delete(deletion) => self.render_deletion(deletion),
            Node::Emphasis(emphasized) => self.render_emphasized(emphasized),
            Node::Link(link) => self.render_link(link),
            Node::LinkReference(linkref) => self.render_linkref(linkref),
            Node::Strong(strong) => self.render_strong(strong),
            Node::Text(text) => self.render_text(text),
            _ => RichText::None,
        }
    }

    fn render_text(&mut self, input: &mdast::Text) -> RichText {
        let text = Text {
            content: input.value.clone(),
            link: None,
        };
        let annotations = Annotations {
            ..Default::default()
        };
        RichText::Text {
            text,
            annotations: Some(annotations),
            plain_text: Some(input.value.clone()),
            href: None,
        }
    }

    fn render_strong(&mut self, strong: &mdast::Strong) -> RichText {
        // One very nice thing we know is that markdown styles are NOT
        // nested. You get emphasis, or you get strong. You don't get both.
        let content: String = strong
            .children
            .iter()
            .filter_map(|xs| match xs {
                Node::Text(ref t) => Some(t.value.clone()),
                _ => None,
            })
            .collect::<Vec<String>>()
            .join("");

        let text = Text {
            content: content.clone(),
            link: None,
        };
        let annotations = Annotations {
            bold: true,
            ..Default::default()
        };
        RichText::Text {
            text,
            annotations: Some(annotations),
            plain_text: Some(content),
            href: None,
        }
    }

    fn render_emphasized(&mut self, emphasized: &mdast::Emphasis) -> RichText {
        let content: String = emphasized
            .children
            .iter()
            .filter_map(|xs| match xs {
                Node::Text(ref t) => Some(t.value.clone()),
                _ => None,
            })
            .collect::<Vec<String>>()
            .join("");

        let text = Text {
            content: content.clone(),
            link: None,
        };
        let annotations = Annotations {
            italic: true,
            ..Default::default()
        };
        RichText::Text {
            text,
            annotations: Some(annotations),
            plain_text: Some(content),
            href: None,
        }
    }

    fn render_link(&mut self, mdlink: &mdast::Link) -> RichText {
        let content: String = mdlink
            .children
            .iter()
            .filter_map(|xs| match xs {
                Node::Text(ref t) => Some(t.value.clone()),
                _ => None,
            })
            .collect::<Vec<String>>()
            .join("");

        let url = if let Some(u) = self.links.get(&mdlink.url) {
            u.clone()
        } else {
            mdlink.url.clone()
        };

        let link = Link { url: url.clone() };
        let text = Text {
            content: content.clone(),
            link: Some(link),
        };
        RichText::Text {
            text,
            annotations: None,
            plain_text: Some(content),
            href: Some(url),
        }
    }

    fn render_linkref(&mut self, linkref: &mdast::LinkReference) -> RichText {
        let content: String = linkref
            .children
            .iter()
            .filter_map(|xs| match xs {
                Node::Text(ref t) => Some(t.value.clone()),
                _ => None,
            })
            .collect::<Vec<String>>()
            .join("");

        let url = if let Some(u) = self.links.get(&linkref.identifier) {
            u.clone()
        } else {
            linkref.identifier.clone()
        };

        let link = Link { url: url.clone() };
        let text = Text {
            content: content.clone(),
            link: Some(link),
        };
        RichText::Text {
            text,
            annotations: None,
            plain_text: Some(content),
            href: Some(url),
        }
    }

    fn render_deletion(&mut self, strike: &mdast::Delete) -> RichText {
        let content: String = strike
            .children
            .iter()
            .filter_map(|xs| match xs {
                Node::Text(ref t) => Some(t.value.clone()),
                _ => None,
            })
            .collect::<Vec<String>>()
            .join("");

        let text = Text {
            content: content.clone(),
            link: None,
        };
        let annotations = Annotations {
            strikethrough: true,
            ..Default::default()
        };
        RichText::Text {
            text,
            annotations: Some(annotations),
            plain_text: Some(content),
            href: None,
        }
    }

    fn render_inline_code(&mut self, inline: &mdast::InlineCode) -> RichText {
        let text = Text {
            content: inline.value.clone(),
            link: None,
        };
        let annotations = Annotations {
            code: true,
            ..Default::default()
        };
        RichText::Text {
            text,
            annotations: Some(annotations),
            plain_text: Some(inline.value.clone()),
            href: None,
        }
    }

    fn render_inline_math(&mut self, math: &mdast::InlineMath) -> RichText {
        let equation = Equation {
            expression: math.value.clone(),
        };
        let annotations = Annotations {
            code: true,
            ..Default::default()
        };

        RichText::Equation {
            equation,
            annotations,
            plain_text: math.value.clone(),
            href: None,
        }
    }

    fn render_quote(&mut self, quote: &mdast::BlockQuote) -> Block {
        let rich_text: Vec<RichText> = quote
            .children
            .iter()
            .map(|xs| self.render_text_node(xs))
            .collect();
        let quote = QuoteValue {
            rich_text,
            color: TextColor::Default,
            children: None,
        };
        Block {
            block_type: BlockType::Quote { quote },
            ..Default::default()
        }
    }

    fn render_footnote(&mut self, footnote: &mdast::FootnoteDefinition) -> Block {
        let rich_text = footnote
            .children
            .iter()
            .map(|xs| self.render_text_node(xs))
            .collect();
        let emoji = Emoji {
            emoji: "🗒️".to_string(),
        };
        let icon = notion_client::objects::block::Icon::Emoji(emoji);
        let callout = CalloutValue {
            rich_text,
            icon,
            color: TextColor::Default,
        };
        Block {
            block_type: BlockType::Callout { callout },
            ..Default::default()
        }
    }

    // I am unsure about this
    fn render_noteref(&mut self, _noteref: &mdast::FootnoteReference) -> Block {
        todo!()
    }

    fn begin_list(&mut self, list: &mdast::List) -> Vec<Block> {
        // list.ordered
        // list.start
        let mut state = State::new();
        state.list = if list.ordered {
            ListVariation::Ordered
        } else {
            ListVariation::Bulleted
        };
        if let Some(start) = list.start {
            state.ordered_start = start;
        }
        state.render_nodes(list.children.as_slice())
    }

    fn render_paragraph(&mut self, para: &mdast::Paragraph) -> Block {
        let rich_text: Vec<RichText> = para
            .children
            .iter()
            .map(|xs| self.render_text_node(xs))
            .collect();
        let paragraph = ParagraphValue {
            rich_text,
            color: Some(TextColor::Default),
            children: None,
        };
        Block {
            block_type: BlockType::Paragraph { paragraph },
            ..Default::default()
        }
    }

    fn render_code(&mut self, fenced: &mdast::Code) -> Block {
        let language = if let Some(langstr) = fenced.lang.as_ref() {
            serde_json::from_str(langstr.as_str()).unwrap_or(Language::PlainText)
        } else {
            Language::PlainText
        };
        let text = Text {
            content: fenced.value.clone(),
            link: None,
        };
        let rich_text = RichText::Text {
            text,
            annotations: None,
            plain_text: Some(fenced.value.clone()),
            href: None,
        };
        let code = CodeValue {
            caption: Vec::new(),
            rich_text: vec![rich_text],
            language,
        };
        Block {
            block_type: BlockType::Code { code },
            ..Default::default()
        }
    }

    fn render_math(&mut self, math: &mdast::Math) -> Block {
        // math.meta
        // math.value
        let equation = EquationValue {
            expression: math.value.clone(),
        };
        Block {
            block_type: BlockType::Equation { equation },
            ..Default::default()
        }
    }

    // This is a hack. There really isn't an equivalent AFAICT.
    fn render_html(&mut self, html: &mdast::Html) -> Block {
        let text = Text {
            content: html.value.clone(),
            link: None,
        };
        let rich_text = RichText::Text {
            text,
            annotations: None,
            plain_text: Some(html.value.clone()),
            href: None,
        };
        let code = CodeValue {
            caption: Vec::new(),
            rich_text: vec![rich_text],
            language: Language::PlainText,
        };
        Block {
            block_type: BlockType::Code { code },
            ..Default::default()
        }
    }

    fn render_image(&mut self, image: &mdast::Image) -> Block {
        todo!()
    }

    fn render_table(&mut self, text: &str) -> Block {
        todo!()
    }

    fn render_table_row(&mut self, text: &str) -> Block {
        todo!()
    }

    fn render_todo(&mut self, text: &str) -> Block {
        todo!()
    }

    fn render_list_item(&mut self, item: &mdast::ListItem) -> Block {
        match self.list {
            ListVariation::None => self.rendered_bullet_li(item),
            ListVariation::Bulleted => self.rendered_bullet_li(item),
            ListVariation::Ordered => self.render_numbered_li(item),
        }
    }

    fn render_numbered_li(&mut self, item: &mdast::ListItem) -> Block {
        let child_blocks = self.render_nodes(item.children.as_slice());
        let rich_text: Vec<RichText> = item
            .children
            .iter()
            .map(|xs| self.render_text_node(xs))
            .collect();
        let numbered_list_item = NumberedListItemValue {
            rich_text,
            color: TextColor::Default,
            children: Some(child_blocks),
        };
        Block {
            block_type: BlockType::NumberedListItem { numbered_list_item },
            ..Default::default()
        }
    }

    fn rendered_bullet_li(&mut self, item: &mdast::ListItem) -> Block {
        let rich_text: Vec<RichText> = item
            .children
            .iter()
            .map(|xs| self.render_text_node(xs))
            .collect();
        let children = self.render_nodes(item.children.as_slice());
        let bulleted_list_item = BulletedListItemValue {
            rich_text,
            color: TextColor::Default,
            children: Some(children),
        };
        Block {
            block_type: BlockType::BulletedListItem { bulleted_list_item },
            ..Default::default()
        }
    }

    fn render_divider(&mut self, _thematic: &mdast::ThematicBreak) -> Block {
        let divider = DividerValue {};
        Block {
            block_type: BlockType::Divider { divider },
            ..Default::default()
        }
    }

    fn render_heading(&mut self, heading: &mdast::Heading) -> Block {
        let rich_text: Vec<RichText> = heading
            .children
            .iter()
            .map(|xs| self.render_text_node(xs))
            .collect();

        let value = HeadingsValue {
            rich_text,
            ..Default::default()
        };
        let block_type = if heading.depth == 1 {
            BlockType::Heading1 { heading_1: value }
        } else if heading.depth == 2 {
            BlockType::Heading2 { heading_2: value }
        } else {
            BlockType::Heading3 { heading_3: value }
        };

        Block {
            block_type,
            ..Default::default()
        }
    }
}
