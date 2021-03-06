/*
 * Copyright (c) 2017 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Generate HTML from the asciidoctor nodes.

use std::io::Write;

use error::Result;
use node::{Attribute, Node};
use node::Attribute::Role;
use node::Node::*;
use node::{Item, Tag, Text};
use self::Html::*;

macro_rules! attr {
    ($( $name:ident = $value:expr ),*) => {{
        let mut attributes = String::new();
        $(
            attributes.push_str(stringify!($name));
            attributes.push_str("=\"");
            attributes.push_str(&$value.to_string());
            attributes.push_str("\"");
        )*
        attributes
    }};
}

type Id = String;

/// Write the resulting HTML code for the specified `node` in the `writer`.
pub fn gen<G: HtmlGen, W: Write>(gen: &mut G, node: &Node, writer: &mut W) -> Result<()> {
    let html = gen.node(node);
    html.write(writer)
}

/// The default HTML generator.
pub struct Generator {
}

/// Genarate an HTML node from a asciidoctor node.
pub trait HtmlGen {
    fn horizontal_rule(&mut self) -> Html {
        hr()
    }

    fn item(&mut self, item: &Item) -> Html {
        match *item {
            Item::Mark(ref text, ref attributes) => self.mark(text, attributes),
            Item::Space => SingleTextNode(" ".to_string()),
            Item::Tag(tag, ref text, ref attributes) => self.tag(tag, text, attributes),
            Item::Word(ref text) => SingleTextNode(text.clone()),
        }
    }

    fn mark(&mut self, text: &Text, attributes: &[Attribute]) -> Html {
        let text = self.text(text);
        if attributes.is_empty() {
            mark(text)
        } else {
            span_a(attributes_to_string(attributes), text)
        }
    }

    fn node(&mut self, node: &Node) -> Html {
        match *node {
            HorizontalRule => self.horizontal_rule(),
            PageBreak => self.page_break(),
            Paragraph(ref text) => self.paragraph(text),
        }
    }

    fn page_break(&mut self) -> Html {
        div_a(
            attr! { style = "page-break-after: always;" },
            Empty
        )
    }

    fn paragraph(&mut self, text: &Text) -> Html {
        let text = self.text(text);
        div_a(
            attr! { class = "paragraph" },
            p(text),
        )
    }

    fn tag(&mut self, tag: Tag, text: &Text, attributes: &[Attribute]) -> Html {
        let text = self.text(text);
        let tag = Tag(tag, attributes_to_string(attributes), Box::new(text));
        if let Some(id) = find_id_attribute(attributes) {
            Seq(Box::new(A(id)), Box::new(tag))
        } else {
            tag
        }
    }

    fn text(&mut self, text: &Text) -> Html {
        let mut texts = vec![];
        for item in &text.items {
            texts.push(self.item(item));
        }
        TextNode(texts)
    }
}

impl HtmlGen for Generator {}

/// Represent an HTML node with its children.
pub enum Html {
    A(Id),
    Div(String, Box<Html>),
    Empty,
    Hr,
    Mark(Box<Html>),
    P(Box<Html>),
    Seq(Box<Html>, Box<Html>),
    SingleTextNode(String),
    Span(String, Box<Html>),
    Tag(Tag, String, Box<Html>),
    TextNode(Vec<Html>),
}

impl Html {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        match *self {
            A(ref id) => tag_a_without_child("a", &attr! { id = id }, writer),
            Div(ref attributes, ref children) => tag_a("div", attributes, children, writer),
            Empty => Ok(()),
            Hr => write_text("<hr/>", writer),
            Mark(ref children) => tag("mark", children, writer),
            P(ref children) => tag("p", children, writer),
            Seq(ref child1, ref child2) => {
                child1.write(writer)?;
                child2.write(writer)
            },
            SingleTextNode(ref text) => write_text(text, writer),
            Span(ref attributes, ref children) => tag_a("span", attributes, children, writer),
            Tag(ref tag, ref attributes, ref children) => tag_a(tag.to_string(), attributes, children, writer),
            TextNode(ref nodes) => {
                for node in nodes {
                    node.write(writer)?;
                }
                Ok(())
            },
        }
    }
}

fn attributes_to_string(attributes: &[Attribute]) -> String {
    let mut string = String::new();
    for attribute in attributes {
        match *attribute {
            Attribute::Id(ref id) => string.push_str(&format!("id=\"{}\"", id)), // TODO: needs space around?
            Role(ref role) => string.push_str(&format!("class=\"{}\"", role)), // TODO: needs space around?
        }
    }
    string
}

/// Create a div element with attributes.
pub fn div_a(attributes: String, children: Html) -> Html {
    Div(attributes, Box::new(children))
}

fn find_id_attribute(attributes: &[Attribute]) -> Option<String> {
    for attribute in attributes {
        if let Attribute::Id(ref id) = *attribute {
            return Some(id.clone());
        }
    }
    None
}

/// Create a hr element.
pub fn hr() -> Html {
    Hr
}

/// Create a mark element.
pub fn mark(children: Html) -> Html {
    Mark(Box::new(children))
}

/// Create a p element.
pub fn p(children: Html) -> Html {
    P(Box::new(children))
}

/// Create a span element.
pub fn span_a(attributes: String, children: Html) -> Html {
    Span(attributes, Box::new(children))
}

fn tag<W: Write>(name: &str, children: &Html, writer: &mut W) -> Result<()> {
    write!(writer, "<{}>", name)?;
    children.write(writer)?;
    write!(writer, "</{}>", name)?;
    Ok(())
}

fn tag_a<W: Write>(name: &str, attributes: &str, children: &Html, writer: &mut W) -> Result<()> {
    write!(writer, "<{} {}>", name, attributes)?;
    children.write(writer)?;
    write!(writer, "</{}>", name)?;
    Ok(())
}

fn tag_a_without_child<W: Write>(name: &str, attributes: &str, writer: &mut W) -> Result<()> {
    write!(writer, "<{} {}>", name, attributes)?;
    write!(writer, "</{}>", name)?;
    Ok(())
}

fn write_text<W: Write>(text: &str, writer: &mut W) -> Result<()> {
    write!(writer, "{}", text)?;
    Ok(())
}
