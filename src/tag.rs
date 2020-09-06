use crate::ast;

pub(crate) enum ConvertedInbandTag<'a> {
    ContainerBlock(ast::ContainerBlockTag<'a>),
    LeafBlock(ast::LeafBlockTag<'a>),
    MarkupBlock(ast::MarkupBlockTag<'a>),
    SurroundingInline(ast::SurroundingInlineTag<'a>),
    ContentInline(ast::ContentInlineTag<'a>),
}

pub(crate) enum ConvertedOutofbandTag<'a> {
    OutOfBandContainer(ast::OutOfBandContainerTag<'a>),
    #[allow(dead_code)]
    OutOfBandLeaf(ast::OutOfBandLeafTag<'a>),
}

pub(crate) enum ConvertedTag<'a> {
    InBand(ConvertedInbandTag<'a>),
    OutOfBand(ConvertedOutofbandTag<'a>),
}

pub(crate) fn convert_tag(tag: pulldown_cmark::Tag<'_>) -> ConvertedTag<'_> {
    use pulldown_cmark::Tag;

    let inband = match tag {
        Tag::Paragraph => ConvertedInbandTag::LeafBlock(ast::LeafBlockTag::Paragraph),
        Tag::Heading(s) => ConvertedInbandTag::LeafBlock(ast::LeafBlockTag::Heading(s)),
        Tag::BlockQuote => ConvertedInbandTag::ContainerBlock(ast::ContainerBlockTag::BlockQuote),
        Tag::CodeBlock(s) => ConvertedInbandTag::LeafBlock(ast::LeafBlockTag::CodeBlock(s)),
        Tag::List(s) => ConvertedInbandTag::ContainerBlock(ast::ContainerBlockTag::List(s)),
        Tag::Item => ConvertedInbandTag::ContainerBlock(ast::ContainerBlockTag::ListItem),
        Tag::FootnoteDefinition(s) => {
            return ConvertedTag::OutOfBand(ConvertedOutofbandTag::OutOfBandContainer(
                ast::OutOfBandContainerTag::FootNoteDef(s),
            ))
        }
        Tag::Table(s) => ConvertedInbandTag::ContainerBlock(ast::ContainerBlockTag::Table(s)),
        Tag::TableHead => ConvertedInbandTag::ContainerBlock(ast::ContainerBlockTag::TableHead),
        Tag::TableRow => ConvertedInbandTag::ContainerBlock(ast::ContainerBlockTag::TableRow),
        Tag::TableCell => ConvertedInbandTag::LeafBlock(ast::LeafBlockTag::TableCell),
        Tag::Emphasis => ConvertedInbandTag::SurroundingInline(ast::SurroundingInlineTag::Emphasis),
        Tag::Strong => ConvertedInbandTag::SurroundingInline(ast::SurroundingInlineTag::Strong),
        Tag::Strikethrough => {
            ConvertedInbandTag::SurroundingInline(ast::SurroundingInlineTag::Strikethrough)
        }
        Tag::Link(a, b, c) => {
            ConvertedInbandTag::SurroundingInline(ast::SurroundingInlineTag::Link(a, b, c))
        }
        Tag::Image(a, b, c) => {
            ConvertedInbandTag::SurroundingInline(ast::SurroundingInlineTag::Image(a, b, c))
        }
    };
    ConvertedTag::InBand(inband)
}

pub(crate) enum UnconvertedTag<'a> {
    SpanEvent(pulldown_cmark::Event<'a>, pulldown_cmark::Event<'a>),
    #[allow(dead_code)]
    SpanEventWithoutLength(pulldown_cmark::Event<'a>, pulldown_cmark::Event<'a>),
    SingleEvent(pulldown_cmark::Event<'a>),
    TransparentEvent,
    Custom(pulldown_cmark::CowStr<'a>),
}

impl<'a> UnconvertedTag<'a> {
    fn new_span_event_with_tag(t: pulldown_cmark::Tag<'a>) -> Self {
        UnconvertedTag::SpanEvent(
            pulldown_cmark::Event::Start(t.clone()),
            pulldown_cmark::Event::End(t),
        )
    }

    #[allow(dead_code)]
    fn new_single_event_with_tag(t: pulldown_cmark::Tag<'a>) -> Self {
        UnconvertedTag::SpanEventWithoutLength(
            pulldown_cmark::Event::Start(t.clone()),
            pulldown_cmark::Event::End(t),
        )
    }

    fn new_single_event(e: pulldown_cmark::Event<'a>) -> Self {
        UnconvertedTag::SingleEvent(e)
    }

    fn new_transparent_span_event() -> Self {
        UnconvertedTag::TransparentEvent
    }

    fn new_custom_event(u: pulldown_cmark::CowStr<'a>) -> Self {
        UnconvertedTag::Custom(u)
    }
}

pub(crate) fn unconvert_tag(t: ConvertedTag<'_>) -> UnconvertedTag<'_> {
    match t {
        ConvertedTag::InBand(t) => match t {
            ConvertedInbandTag::ContainerBlock(t) => match t {
                ast::ContainerBlockTag::BlockQuote => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::BlockQuote)
                }
                ast::ContainerBlockTag::List(u) => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::List(u))
                }
                ast::ContainerBlockTag::ListItem => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::Item)
                }
                ast::ContainerBlockTag::Table(u) => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::Table(u))
                }
                ast::ContainerBlockTag::TableHead => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::TableHead)
                }
                ast::ContainerBlockTag::TableRow => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::TableRow)
                }
                ast::ContainerBlockTag::Custom(u) => UnconvertedTag::new_custom_event(u),
            },
            ConvertedInbandTag::LeafBlock(t) => match t {
                ast::LeafBlockTag::Paragraph => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::Paragraph)
                }
                ast::LeafBlockTag::Heading(u) => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::Heading(u))
                }
                ast::LeafBlockTag::CodeBlock(u) => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::CodeBlock(u))
                }
                ast::LeafBlockTag::TableCell => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::TableCell)
                }
                ast::LeafBlockTag::Html => UnconvertedTag::new_transparent_span_event(),
                ast::LeafBlockTag::Custom(u) => UnconvertedTag::new_custom_event(u),
            },
            ConvertedInbandTag::MarkupBlock(t) => match t {
                ast::MarkupBlockTag::Rule => {
                    UnconvertedTag::new_single_event(pulldown_cmark::Event::Rule)
                }
                ast::MarkupBlockTag::Custom(u) => UnconvertedTag::new_custom_event(u),
            },
            ConvertedInbandTag::SurroundingInline(t) => match t {
                ast::SurroundingInlineTag::Emphasis => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::Emphasis)
                }
                ast::SurroundingInlineTag::Strong => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::Strong)
                }
                ast::SurroundingInlineTag::Strikethrough => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::Strikethrough)
                }
                ast::SurroundingInlineTag::Link(a, b, c) => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::Link(a, b, c))
                }
                ast::SurroundingInlineTag::Image(a, b, c) => {
                    UnconvertedTag::new_span_event_with_tag(pulldown_cmark::Tag::Image(a, b, c))
                }
                ast::SurroundingInlineTag::Custom(u) => UnconvertedTag::new_custom_event(u),
            },
            ConvertedInbandTag::ContentInline(t) => match t {
                ast::ContentInlineTag::Text(u) => {
                    UnconvertedTag::new_single_event(pulldown_cmark::Event::Text(u))
                }
                ast::ContentInlineTag::Code(u) => {
                    UnconvertedTag::new_single_event(pulldown_cmark::Event::Code(u))
                }
                ast::ContentInlineTag::RawHtml(u) => {
                    UnconvertedTag::new_single_event(pulldown_cmark::Event::Html(u))
                }
                ast::ContentInlineTag::FootnoteRef(u) => {
                    UnconvertedTag::new_single_event(pulldown_cmark::Event::FootnoteReference(u))
                }
                ast::ContentInlineTag::TaskListMarker(u) => {
                    UnconvertedTag::new_single_event(pulldown_cmark::Event::TaskListMarker(u))
                }
                ast::ContentInlineTag::SoftBreak => {
                    UnconvertedTag::new_single_event(pulldown_cmark::Event::SoftBreak)
                }
                ast::ContentInlineTag::HardBreak => {
                    UnconvertedTag::new_single_event(pulldown_cmark::Event::HardBreak)
                }
                ast::ContentInlineTag::Custom(u) => UnconvertedTag::new_custom_event(u),
            },
        },
        ConvertedTag::OutOfBand(t) => match t {
            ConvertedOutofbandTag::OutOfBandContainer(t) => match t {
                ast::OutOfBandContainerTag::FootNoteDef(u) => {
                    UnconvertedTag::new_span_event_with_tag(
                        pulldown_cmark::Tag::FootnoteDefinition(u),
                    )
                }
                ast::OutOfBandContainerTag::Custom(u) => UnconvertedTag::new_custom_event(u),
            },
            ConvertedOutofbandTag::OutOfBandLeaf(t) => match t {
                ast::OutOfBandLeafTag::Custom(u) => UnconvertedTag::new_custom_event(u),
            },
        },
    }
}

pub(crate) fn is_token_inline<'a>(e: &pulldown_cmark::Event<'a>) -> bool {
    match e {
        pulldown_cmark::Event::Start(tag) | pulldown_cmark::Event::End(tag) => match tag {
            pulldown_cmark::Tag::Paragraph
            | pulldown_cmark::Tag::Heading(_)
            | pulldown_cmark::Tag::BlockQuote
            | pulldown_cmark::Tag::CodeBlock(_)
            | pulldown_cmark::Tag::List(_)
            | pulldown_cmark::Tag::Item
            | pulldown_cmark::Tag::FootnoteDefinition(_)
            | pulldown_cmark::Tag::Table(_)
            | pulldown_cmark::Tag::TableHead
            | pulldown_cmark::Tag::TableRow
            | pulldown_cmark::Tag::TableCell => false,
            pulldown_cmark::Tag::Emphasis
            | pulldown_cmark::Tag::Strong
            | pulldown_cmark::Tag::Strikethrough
            | pulldown_cmark::Tag::Link(_, _, _)
            | pulldown_cmark::Tag::Image(_, _, _) => true,
        },
        pulldown_cmark::Event::Text(_)
        | pulldown_cmark::Event::Code(_)
        | pulldown_cmark::Event::Html(_)
        | pulldown_cmark::Event::FootnoteReference(_)
        | pulldown_cmark::Event::SoftBreak
        | pulldown_cmark::Event::HardBreak
        | pulldown_cmark::Event::TaskListMarker(_) => true,
        pulldown_cmark::Event::Rule => false,
    }
}
