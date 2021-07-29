use std::fmt;

use once_cell::sync::Lazy;
use regex::Regex;
use sxd_document;
use sxd_document::dom::{ChildOfElement, Element};
use sxd_xpath;
use sxd_xpath::nodeset::Nodeset;


static WHITESPACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(
    "\\s+"
).expect("failed to parse regex"));


#[derive(Debug, PartialEq)]
pub(crate) enum ParsingError {
    XmlParsing(sxd_document::parser::Error),
    XpathParsing(sxd_xpath::ParserError),
    XpathExecution(sxd_xpath::ExecutionError),
    ExpectedNodeset(String),
    ExpectedSingleNode(usize),
    ExpectedElement(String),
}
impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::XmlParsing(e)
                => write!(f, "XML parsing error: {}", e),
            Self::XpathParsing(e)
                => write!(f, "XPath parsing error: {}", e),
            Self::XpathExecution(e)
                => write!(f, "XPath execution error: {}", e),
            Self::ExpectedNodeset(val)
                => write!(f, "expected nodeset, got {}", val),
            Self::ExpectedSingleNode(len)
                => write!(f, "expected single node, got {} nodes", len),
            Self::ExpectedElement(node)
                => write!(f, "expected element, got {}", node),
        }
    }
}
impl std::error::Error for ParsingError {
}
impl From<sxd_document::parser::Error> for ParsingError {
    fn from(e: sxd_document::parser::Error) -> Self {
        Self::XmlParsing(e)
    }
}
impl From<sxd_xpath::ParserError> for ParsingError {
    fn from(e: sxd_xpath::ParserError) -> Self {
        Self::XpathParsing(e)
    }
}
impl From<sxd_xpath::ExecutionError> for ParsingError {
    fn from(e: sxd_xpath::ExecutionError) -> Self {
        Self::XpathExecution(e)
    }
}

fn to_nodeset<'a>(val: sxd_xpath::Value<'a>) -> Result<Nodeset<'a>, ParsingError> {
    if let sxd_xpath::Value::Nodeset(ns) = val {
        Ok(ns)
    } else {
        Err(ParsingError::ExpectedNodeset(format!("{:?}", val)))
    }
}
fn to_node<'a>(val: sxd_xpath::Value<'a>) -> Result<sxd_xpath::nodeset::Node<'a>, ParsingError> {
    let nodeset = to_nodeset(val)?;
    if nodeset.size() != 1 {
        Err(ParsingError::ExpectedSingleNode(nodeset.size()))
    } else {
        Ok(nodeset.document_order_first().unwrap())
    }
}
fn to_element<'a>(val: sxd_xpath::Value<'a>) -> Result<Element<'a>, ParsingError> {
    let node = to_node(val)?;
    node.element()
        .ok_or(ParsingError::ExpectedElement(format!("{:?}", node)))
}


fn iterate_element(element: &Element) -> String {
    let mut ret = String::new();

    for child in &element.children() {
        match child {
            ChildOfElement::Element(elem) => {
                let elem_name = elem.name().local_part();
                if elem_name.len() == 2 && elem_name.starts_with("h") && elem_name.chars().nth(1).unwrap().is_ascii_digit() {
                    // <h1> etc. don't tend to contain full sentences
                    // => skip
                } else {
                    match elem.name().local_part().to_lowercase().as_str() {
                        "ul"|"ol"|"dl"|"li" => {
                            // lists tend to contain fragments instead of sentences
                            // => skip
                        },
                        "math"|"chem"|"timeline"|"syntaxhighlight"|"hiero"|"inputbox"|"score"|"graph"|"categorytree" => {
                            // this is anything but the natural language we're trying to process
                            // => skip
                        },
                        "sup"|"sub" => {
                            // mostly references, and otherwise not very natural-language either
                            // => skip
                        },
                        "a" => {
                            // ensure it is not a category link
                            let descend = if let Some(href) = elem.attribute_value("href") {
                                !href.starts_with("./Kategorie:")
                            } else {
                                true
                            };

                            if descend {
                                let children_string = iterate_element(elem);
                                ret.push_str(&children_string);
                            }
                        },
                        "table" => {
                            // tables can contain both sentences and fragmentary text
                            // => skip
                        },
                        "span" => {
                            // ensure it's not an image
                            let descend = if let Some(type_of) = elem.attribute_value("typeof") {
                                !type_of.contains("mw:Image")
                            } else {
                                true
                            };

                            if descend {
                                let children_string = iterate_element(elem);
                                ret.push_str(&children_string);
                            }
                        },
                        _other => {
                            let children_string = iterate_element(elem);
                            ret.push_str(&children_string);
                        },
                    }
                }
            },
            ChildOfElement::Text(t) => {
                ret.push_str(t.text());
            },
            ChildOfElement::Comment(_comment) => {},
            ChildOfElement::ProcessingInstruction(_instr) => {},
        }
    }

    ret
}


pub(crate) fn xhtml_to_plain(mut xhtml: &str) -> Result<String, ParsingError> {
    if xhtml.starts_with("<!DOCTYPE html>") {
        xhtml = &xhtml["<!DOCTYPE html>".len()..];
    }
    let xhtml_replaced = xhtml
        .replace(" xmlns=\"http://www.w3.org/2000/xmlns/\"", "")
        .replace(" xmlns='http://www.w3.org/2000/xmlns/'", "");

    // parse
    let parsed = sxd_document::parser::parse(&xhtml_replaced)?;
    let document = parsed.as_document();
    let xpath_factory = sxd_xpath::Factory::new();
    let xpath_context = sxd_xpath::Context::new();

    // get body
    let body_xpath = xpath_factory.build("/html/body")?
        .expect("XPath actually generated");
    let body_element = to_element(body_xpath.evaluate(&xpath_context, document.root())?)?;

    let mut plaintext = iterate_element(&body_element);
    plaintext = WHITESPACE_RE.replace_all(&plaintext, " ")
        .trim()
        .to_owned();
    Ok(plaintext)
}
