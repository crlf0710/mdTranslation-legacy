use crate::ast;
use crate::tag::{
    convert_tag, is_token_inline, ConvertedInbandTag, ConvertedOutofbandTag, ConvertedTag,
};
use core::mem;
use thiserror::Error;

#[derive(Clone, Error, Debug)]
#[error("FromTokensError")]
pub struct FromTokensError;

enum InBandContext<'a, 'b> {
    ChildrenBlocks {
        blocks: &'b mut ast::BlockNodeList<'a>,
    },
    ChildrenInlines {
        inlines: &'b mut ast::InlineNodeList<'a>,
    },
    ChildrenNone {},
}

enum OutOfBandContext<'a, 'b> {
    OutOfBand {
        outofbands: &'b mut ast::OutOfBandNodeList<'a>,
    },
}

impl<'a, 'b> OutOfBandContext<'a, 'b> {
    fn reborrow<'c>(&'c mut self) -> OutOfBandContext<'a, 'c> {
        match self {
            OutOfBandContext::OutOfBand { outofbands } => {
                OutOfBandContext::OutOfBand { outofbands }
            }
        }
    }
}

fn read_token_or_token_region<'a>(
    tokens: &mut core::iter::Peekable<impl Iterator<Item = pulldown_cmark::Event<'a>>>,
    output: &mut Vec<pulldown_cmark::Event<'a>>,
) -> Result<(), FromTokensError> {
    let mut region_stack = Vec::new();
    while let Some(token) = tokens.next() {
        match &token {
            pulldown_cmark::Event::Start(tag) => {
                region_stack.push(tag.clone());
            }
            pulldown_cmark::Event::End(tag) => {
                let stack_item = region_stack.pop();
                if stack_item.as_ref() != Some(tag) {
                    return Err(FromTokensError);
                }
            }
            _ => {}
        }
        output.push(token);
        if region_stack.is_empty() {
            return Ok(());
        }
    }
    Ok(())
}

fn load_ast_nodes<'a, 'b>(
    tokens: &mut core::iter::Peekable<impl Iterator<Item = pulldown_cmark::Event<'a>>>,
    terminator: Option<pulldown_cmark::Tag<'a>>,
    mut in_band_ctx: InBandContext<'a, 'b>,
    mut out_of_band_ctx: OutOfBandContext<'a, 'b>,
) -> Result<(), FromTokensError> {
    use pulldown_cmark::Event;

    loop {
        let mut peek_token = tokens.peek();
        if peek_token.is_none() {
            if terminator.is_none() {
                break;
            } else {
                return Err(FromTokensError);
            }
        }
        if let (
            Some(Event::Start(pulldown_cmark::Tag::Item)),
            InBandContext::ChildrenBlocks { blocks },
        ) = (&peek_token, &mut in_band_ctx)
        {
            // Workaround https://github.com/raphlinus/pulldown-cmark/issues/475
            let mut children = Vec::new();
            let mut temporary_inline_contents = Vec::new();
            let _ = tokens.next();
            'specialized_processing_item_node: loop {
                let peek_token = tokens.peek();
                if let Some(token) = peek_token {
                    if is_token_inline(&token) {
                        let mut temporary_buffer = Vec::new();
                        read_token_or_token_region(tokens, &mut temporary_buffer)?;
                        let inner_in_band_ctx = InBandContext::ChildrenInlines {
                            inlines: &mut temporary_inline_contents,
                        };
                        load_ast_nodes(
                            &mut temporary_buffer.into_iter().peekable(),
                            None,
                            inner_in_band_ctx,
                            out_of_band_ctx.reborrow(),
                        )?;
                    } else {
                        if !temporary_inline_contents.is_empty() {
                            let paragraph_contents = mem::take(&mut temporary_inline_contents);
                            let paragraph = ast::BlockNode::Leaf {
                                tag: ast::LeafBlockTag::Paragraph,
                                contents: paragraph_contents,
                            };
                            children.push(paragraph);
                        }
                        if let Some(Event::End(t)) = peek_token {
                            if !matches!(t, pulldown_cmark::Tag::Item) {
                                return Err(FromTokensError);
                            }
                            let _ = tokens.next();
                            break 'specialized_processing_item_node;
                        }
                        let mut temporary_buffer = Vec::new();
                        read_token_or_token_region(tokens, &mut temporary_buffer)?;
                        let inner_in_band_ctx = InBandContext::ChildrenBlocks {
                            blocks: &mut children,
                        };
                        load_ast_nodes(
                            &mut temporary_buffer.into_iter().peekable(),
                            None,
                            inner_in_band_ctx,
                            out_of_band_ctx.reborrow(),
                        )?;
                    }
                } else {
                    return Err(FromTokensError);
                }
            }
            let item_block = ast::BlockNode::Container {
                tag: ast::ContainerBlockTag::ListItem,
                children,
            };
            blocks.push(item_block);
            continue;
        } else if let (Some(Event::Html(..)), InBandContext::ChildrenBlocks { blocks }) =
            (&peek_token, &mut in_band_ctx)
        {
            // Workaround https://github.com/raphlinus/pulldown-cmark/issues/473
            let mut contents = Vec::new();
            while let Some(Event::Html(..)) = peek_token {
                if let Some(Event::Html(s)) = tokens.next() {
                    contents.push(ast::InlineNode::Content {
                        tag: ast::ContentInlineTag::RawHtml(s),
                    });
                }
                peek_token = tokens.peek();
            }
            let html_block = ast::BlockNode::Leaf {
                tag: ast::LeafBlockTag::Html,
                contents,
            };
            blocks.push(html_block);
            continue;
        }
        match tokens.next().unwrap() {
            Event::Start(tag) => match convert_tag(tag.clone()) {
                ConvertedTag::InBand(in_band_tag) => match &mut in_band_ctx {
                    InBandContext::ChildrenBlocks { blocks } => {
                        let mut new_inband_node;
                        match in_band_tag {
                            ConvertedInbandTag::ContainerBlock(tag) => {
                                new_inband_node = ast::BlockNode::Container {
                                    tag,
                                    children: Vec::new(),
                                };
                            }
                            ConvertedInbandTag::LeafBlock(tag) => {
                                new_inband_node = ast::BlockNode::Leaf {
                                    tag,
                                    contents: Vec::new(),
                                };
                            }
                            _ => return Err(FromTokensError),
                        }
                        let inner_ib_ctx = match &mut new_inband_node {
                            ast::BlockNode::Container { children, .. } => {
                                InBandContext::ChildrenBlocks { blocks: children }
                            }
                            ast::BlockNode::Leaf { contents, .. } => {
                                InBandContext::ChildrenInlines { inlines: contents }
                            }
                            ast::BlockNode::Markup { .. } => unreachable!(),
                        };
                        load_ast_nodes(
                            tokens,
                            Some(tag),
                            inner_ib_ctx,
                            out_of_band_ctx.reborrow(),
                        )?;
                        blocks.push(new_inband_node);
                    }
                    InBandContext::ChildrenInlines { inlines } => {
                        let mut new_inband_node;
                        match in_band_tag {
                            ConvertedInbandTag::SurroundingInline(tag) => {
                                new_inband_node = ast::InlineNode::Surrounding {
                                    tag,
                                    contents: Vec::new(),
                                };
                            }
                            ConvertedInbandTag::ContentInline(tag) => {
                                new_inband_node = ast::InlineNode::Content { tag };
                            }
                            _ => return Err(FromTokensError),
                        }
                        let inner_ib_ctx = match &mut new_inband_node {
                            ast::InlineNode::Surrounding { contents, .. } => {
                                InBandContext::ChildrenInlines { inlines: contents }
                            }
                            ast::InlineNode::Content { .. } => InBandContext::ChildrenNone {},
                        };
                        load_ast_nodes(
                            tokens,
                            Some(tag),
                            inner_ib_ctx,
                            out_of_band_ctx.reborrow(),
                        )?;
                        inlines.push(new_inband_node);
                    }
                    InBandContext::ChildrenNone {} => {
                        return Err(FromTokensError);
                    }
                },
                ConvertedTag::OutOfBand(out_of_band_tag) => {
                    let mut inner_oob_ctx_list = Vec::new();
                    let inner_oob_ctx = OutOfBandContext::OutOfBand {
                        outofbands: &mut inner_oob_ctx_list,
                    };
                    let mut new_oob_node;
                    match out_of_band_tag {
                        ConvertedOutofbandTag::OutOfBandContainer(tag) => {
                            let out_of_band_tag = tag;
                            new_oob_node = ast::OutOfBandNode::OutOfBandContainer {
                                tag: out_of_band_tag,
                                children: Vec::new(),
                            };
                        }
                        ConvertedOutofbandTag::OutOfBandLeaf(tag) => {
                            let out_of_band_tag = tag;
                            new_oob_node = ast::OutOfBandNode::OutOfBandLeaf {
                                tag: out_of_band_tag,
                                contents: Vec::new(),
                            };
                        }
                    }
                    let inner_ib_ctx = match &mut new_oob_node {
                        ast::OutOfBandNode::OutOfBandContainer { children, .. } => {
                            InBandContext::ChildrenBlocks { blocks: children }
                        }
                        ast::OutOfBandNode::OutOfBandLeaf { contents, .. } => {
                            InBandContext::ChildrenInlines { inlines: contents }
                        }
                    };
                    load_ast_nodes(tokens, Some(tag), inner_ib_ctx, inner_oob_ctx)?;
                    let existing_oob_list = match &mut out_of_band_ctx {
                        OutOfBandContext::OutOfBand { outofbands } => outofbands,
                    };
                    existing_oob_list.push(new_oob_node);
                    existing_oob_list.extend(inner_oob_ctx_list);
                }
            },
            Event::End(tag) => {
                if let Some(expected_tag) = &terminator {
                    if *expected_tag == tag {
                        break;
                    } else {
                        return Err(FromTokensError);
                    }
                } else {
                    return Err(FromTokensError);
                }
            }
            Event::Rule => match &mut in_band_ctx {
                InBandContext::ChildrenBlocks { blocks } => blocks.push(ast::BlockNode::Markup {
                    tag: ast::MarkupBlockTag::Rule,
                }),
                _ => return Err(FromTokensError),
            },
            Event::Text(s) => match &mut in_band_ctx {
                InBandContext::ChildrenInlines { inlines } => {
                    inlines.push(ast::InlineNode::Content {
                        tag: ast::ContentInlineTag::Text(s),
                    })
                }
                _ => return Err(FromTokensError),
            },
            Event::Code(s) => match &mut in_band_ctx {
                InBandContext::ChildrenInlines { inlines } => {
                    inlines.push(ast::InlineNode::Content {
                        tag: ast::ContentInlineTag::Code(s),
                    })
                }
                _ => return Err(FromTokensError),
            },
            Event::Html(s) => match &mut in_band_ctx {
                InBandContext::ChildrenInlines { inlines } => {
                    inlines.push(ast::InlineNode::Content {
                        tag: ast::ContentInlineTag::RawHtml(s),
                    })
                }
                _ => unreachable!(),
            },
            Event::FootnoteReference(s) => match &mut in_band_ctx {
                InBandContext::ChildrenInlines { inlines } => {
                    inlines.push(ast::InlineNode::Content {
                        tag: ast::ContentInlineTag::FootnoteRef(s),
                    })
                }
                _ => return Err(FromTokensError),
            },
            Event::SoftBreak => match &mut in_band_ctx {
                InBandContext::ChildrenInlines { inlines } => {
                    inlines.push(ast::InlineNode::Content {
                        tag: ast::ContentInlineTag::SoftBreak,
                    })
                }
                _ => return Err(FromTokensError),
            },
            Event::HardBreak => match &mut in_band_ctx {
                InBandContext::ChildrenInlines { inlines } => {
                    inlines.push(ast::InlineNode::Content {
                        tag: ast::ContentInlineTag::HardBreak,
                    })
                }
                _ => return Err(FromTokensError),
            },
            Event::TaskListMarker(s) => match &mut in_band_ctx {
                InBandContext::ChildrenInlines { inlines } => {
                    inlines.push(ast::InlineNode::Content {
                        tag: ast::ContentInlineTag::TaskListMarker(s),
                    })
                }
                _ => return Err(FromTokensError),
            },
        }
    }
    Ok(())
}

pub fn cmark_ast_from_tokens(
    tokens: pulldown_cmark::Parser<'_>,
) -> Result<ast::Document<'_>, FromTokensError> {
    let mut doc: ast::Document = Default::default();
    let mut tokens = tokens.peekable();
    let in_band_ctx = InBandContext::ChildrenBlocks {
        blocks: &mut doc.blocks,
    };
    let out_of_band_ctx = OutOfBandContext::OutOfBand {
        outofbands: &mut doc.outofbands,
    };
    load_ast_nodes(&mut tokens, None, in_band_ctx, out_of_band_ctx)?;
    if tokens.next().is_some() {
        return Err(FromTokensError);
    }
    Ok(doc)
}
