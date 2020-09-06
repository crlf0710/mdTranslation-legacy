use crate::ast;
use core::mem;

pub(crate) const SENTENCE_TAG: pulldown_cmark::CowStr<'static> =
    pulldown_cmark::CowStr::Borrowed("Sentence");

fn cowstr_substr<'a>(
    cowstr: &pulldown_cmark::CowStr<'a>,
    range: core::ops::Range<usize>,
) -> pulldown_cmark::CowStr<'a> {
    match cowstr {
        pulldown_cmark::CowStr::Boxed(_) | pulldown_cmark::CowStr::Inlined(_) => {
            pulldown_cmark::CowStr::Boxed(cowstr.as_ref()[range].to_string().into_boxed_str())
        }
        pulldown_cmark::CowStr::Borrowed(s) => pulldown_cmark::CowStr::Borrowed(&s[range]),
    }
}

fn split_text<'a>(
    input: &pulldown_cmark::CowStr<'a>,
    output: &mut ast::InlineNodeList<'a>,
    proposed_split_positions: &[usize],
    accumulative_length: usize,
) {
    let total_len = input.as_ref().len();
    let split_pos_iter = proposed_split_positions
        .iter()
        .take_while(|pos| accumulative_length + *pos < total_len);
    let mut start_pos = 0;
    for &split_pos in split_pos_iter {
        let split_pos = split_pos.checked_sub(accumulative_length).unwrap();
        let new_text = cowstr_substr(&input, start_pos..split_pos);
        output.push(ast::InlineNode::Content {
            tag: ast::ContentInlineTag::Text(new_text),
        });
        start_pos = split_pos;
    }
    let rest_text = cowstr_substr(&input, start_pos..total_len);
    output.push(ast::InlineNode::Content {
        tag: ast::ContentInlineTag::Text(rest_text),
    });
}

fn split_inline_node<'a>(
    mut input: ast::InlineNode<'a>,
    output: &mut ast::InlineNodeList<'a>,
    proposed_split_positions: &mut &[usize],
    accumulative_length: &mut usize,
) {
    use crate::textualize::*;
    let mut textualize_result = String::new();
    'custom_processing: loop {
        let mut splitted_nodes = Vec::new();
        match &mut input {
            ast::InlineNode::Surrounding { tag, contents } => match tag {
                ast::SurroundingInlineTag::Emphasis
                | ast::SurroundingInlineTag::Strong
                | ast::SurroundingInlineTag::Strikethrough => {
                    let mut inner_proposed_split_positions = *proposed_split_positions;
                    let mut inner_accumulative_length = *accumulative_length;
                    split_inlines(
                        contents,
                        &mut inner_proposed_split_positions,
                        &mut inner_accumulative_length,
                    );
                    regroup_inlines(
                        mem::take(contents),
                        &mut splitted_nodes,
                        |nodes| ast::InlineNode::Surrounding {
                            tag: tag.clone(),
                            contents: nodes,
                        },
                        |node| {
                            if let ast::InlineNode::Surrounding {
                                tag: current_tag, ..
                            } = node
                            {
                                current_tag == tag
                            } else {
                                false
                            }
                        },
                        *proposed_split_positions,
                        *accumulative_length,
                    );
                }
                ast::SurroundingInlineTag::Link(_, _, _)
                | ast::SurroundingInlineTag::Image(_, _, _)
                | ast::SurroundingInlineTag::Custom(_) => {
                    textualize_inline_node(&input, &mut textualize_result);
                    output.push(input);
                    break 'custom_processing;
                }
            },
            ast::InlineNode::Content { tag } => match tag {
                ast::ContentInlineTag::Text(s) => {
                    let inner_proposed_split_positions = *proposed_split_positions;
                    let inner_accumulative_length = *accumulative_length;
                    split_text(
                        s,
                        &mut splitted_nodes,
                        inner_proposed_split_positions,
                        inner_accumulative_length,
                    );
                }
                ast::ContentInlineTag::Code(_)
                | ast::ContentInlineTag::RawHtml(_)
                | ast::ContentInlineTag::FootnoteRef(_)
                | ast::ContentInlineTag::TaskListMarker(_)
                | ast::ContentInlineTag::SoftBreak
                | ast::ContentInlineTag::HardBreak
                | ast::ContentInlineTag::Custom(_) => {
                    textualize_inline_node(&input, &mut textualize_result);
                    output.push(input);
                    break 'custom_processing;
                }
            },
        }
        textualize_inline_list(&splitted_nodes, &mut textualize_result);
        output.extend(splitted_nodes);
        break 'custom_processing;
    }
    let new_accumulative_length = *accumulative_length + textualize_result.len();
    let mut last_skipped_index = None;
    for (idx, pos) in proposed_split_positions.iter().enumerate() {
        if *pos <= new_accumulative_length {
            last_skipped_index = Some(idx)
        } else {
            break;
        }
    }
    if let Some(last_skipped_index) = last_skipped_index {
        *proposed_split_positions = &proposed_split_positions[last_skipped_index + 1..]
    }
    *accumulative_length = new_accumulative_length;
}

fn split_inlines<'a>(
    inlines: &mut ast::InlineNodeList<'a>,
    proposed_split_positions: &mut &[usize],
    accumulative_length: &mut usize,
) {
    if proposed_split_positions.is_empty() {
        return;
    }
    let input = mem::take(inlines);
    for inline in input {
        let mut split_result = Vec::new();
        split_inline_node(
            inline,
            &mut split_result,
            proposed_split_positions,
            accumulative_length,
        );
        inlines.extend(split_result);
    }
}

fn is_split_point(pos: usize, proposed_split_positions: &[usize]) -> bool {
    proposed_split_positions.iter().any(|x| *x == pos)
}

fn regroup_inlines<'a>(
    input: ast::InlineNodeList<'a>,
    output: &mut ast::InlineNodeList<'a>,
    wrapper_fn: impl Fn(ast::InlineNodeList<'a>) -> ast::InlineNode,
    nowrap_check_fn: impl Fn(&ast::InlineNode<'a>) -> bool,
    proposed_split_positions: &[usize],
    mut accumulative_length: usize,
) {
    use crate::textualize::textualize_inline_node;
    let mut intermediate_list = Vec::new();
    let mut intermediate_length = 0;
    for node in input {
        let mut textualize_str = String::new();
        textualize_inline_node(&node, &mut textualize_str);
        if textualize_str.is_empty() {
            intermediate_list.push(node);
            continue;
        }
        if is_split_point(
            accumulative_length + intermediate_length,
            proposed_split_positions,
        ) {
            assert!(!intermediate_list.is_empty());
            let node = if intermediate_list.len() == 1 {
                if nowrap_check_fn(intermediate_list.first().unwrap()) {
                    intermediate_list.into_iter().next().unwrap()
                } else {
                    wrapper_fn(intermediate_list)
                }
            } else {
                wrapper_fn(intermediate_list)
            };
            output.push(node);
            accumulative_length += intermediate_length;
            intermediate_list = Vec::new();
            intermediate_length = 0;
        }
        intermediate_list.push(node);
        intermediate_length += textualize_str.len();
    }
    if !intermediate_list.is_empty() {
        let node = if intermediate_list.len() == 1 {
            if nowrap_check_fn(intermediate_list.first().unwrap()) {
                intermediate_list.into_iter().next().unwrap()
            } else {
                wrapper_fn(intermediate_list)
            }
        } else {
            wrapper_fn(intermediate_list)
        };
        output.push(node);
    }
}

fn perform_sentence_segment_for_leaf_contents<'a>(inlines: &mut ast::InlineNodeList<'a>) {
    use crate::textualize::textualize_inline_list;
    use unicode_segmentation::UnicodeSegmentation;
    let mut textualize_result = String::new();
    textualize_inline_list(inlines, &mut textualize_result);
    let offsets: Vec<_> = textualize_result
        .unicode_sentences()
        .skip(1)
        .map(|sentence_str| sentence_str.as_ptr() as usize - textualize_result.as_ptr() as usize)
        .collect();
    split_inlines(inlines, &mut &offsets[..], &mut 0);
    let regroup_input = mem::take(inlines);
    regroup_inlines(
        regroup_input,
        inlines,
        |nodes| ast::InlineNode::Surrounding {
            tag: ast::SurroundingInlineTag::Custom(SENTENCE_TAG),
            contents: nodes,
        },
        |node| {
            if let ast::InlineNode::Surrounding {
                tag: current_tag, ..
            } = node
            {
                *current_tag == ast::SurroundingInlineTag::Custom(SENTENCE_TAG)
            } else {
                false
            }
        },
        &offsets[..],
        0,
    );
}

fn perform_sentence_segment_for_block_node<'a>(block: &mut ast::BlockNode<'a>) {
    match block {
        ast::BlockNode::Container { children, .. } => {
            for block in children.iter_mut() {
                perform_sentence_segment_for_block_node(block);
            }
        }
        ast::BlockNode::Leaf { contents, .. } => {
            perform_sentence_segment_for_leaf_contents(contents);
        }
        ast::BlockNode::Markup { .. } => {
            // do nothing
        }
    }
}

fn perform_sentence_segment_for_out_of_band_node<'a>(oob: &mut ast::OutOfBandNode<'a>) {
    match oob {
        ast::OutOfBandNode::OutOfBandContainer { children, .. } => {
            for block in children.iter_mut() {
                perform_sentence_segment_for_block_node(block);
            }
        }
        ast::OutOfBandNode::OutOfBandLeaf { contents, .. } => {
            perform_sentence_segment_for_leaf_contents(contents);
        }
    }
}

impl<'a> ast::Document<'a> {
    pub fn perform_sentence_segment(&mut self) {
        for block in self.blocks.iter_mut() {
            perform_sentence_segment_for_block_node(block);
        }

        for outofband in self.outofbands.iter_mut() {
            perform_sentence_segment_for_out_of_band_node(outofband);
        }
    }
}
