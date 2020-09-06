use crate::ast;

pub fn textualize_inline_list(inlines: &ast::InlineNodeList<'_>, textualize_result: &mut String) {
    for inline in inlines {
        textualize_inline_node(inline, textualize_result);
    }
}

pub fn textualize_inline_node(node: &ast::InlineNode<'_>, textualize_result: &mut String) {
    match node {
        ast::InlineNode::Surrounding { tag, contents } => match tag {
            ast::SurroundingInlineTag::Emphasis
            | ast::SurroundingInlineTag::Strong
            | ast::SurroundingInlineTag::Strikethrough => {
                *textualize_result += "(";
                textualize_inline_list(contents, textualize_result);
                *textualize_result += ")";
            }
            ast::SurroundingInlineTag::Link(_, _, _) => {
                *textualize_result += "(link)";
            }
            ast::SurroundingInlineTag::Image(_, _, _) => {
                *textualize_result += "(image)";
            }
            ast::SurroundingInlineTag::Custom(_) => {
                textualize_inline_list(contents, textualize_result);
            }
        },
        ast::InlineNode::Content { tag } => match tag {
            ast::ContentInlineTag::Text(s) => {
                *textualize_result += s.as_ref();
            }
            ast::ContentInlineTag::Code(_) => {
                *textualize_result += "(code)";
            }
            ast::ContentInlineTag::RawHtml(_) => {
                *textualize_result += "(raw html)";
            }
            ast::ContentInlineTag::FootnoteRef(_) => {
                *textualize_result += "(ref)";
            }
            ast::ContentInlineTag::TaskListMarker(_) => {
                *textualize_result += "(marker)";
            }
            ast::ContentInlineTag::SoftBreak => {
                *textualize_result += " ";
            }
            ast::ContentInlineTag::HardBreak => {
                *textualize_result += "\n";
            }
            ast::ContentInlineTag::Custom(s) => {
                *textualize_result += "(";
                *textualize_result += s.as_ref();
                *textualize_result += ")";
            }
        },
    }
}
