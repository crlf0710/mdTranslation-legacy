use crate::ast;
use crate::clause;
use crate::tag::{
    unconvert_tag, ConvertedInbandTag, ConvertedOutofbandTag, ConvertedTag, UnconvertedTag,
};
use alloc::collections::VecDeque;

impl<'a> ast::Document<'a> {
    pub fn into_tokens(self) -> EventIter<'a> {
        let iter1 = self.blocks.into_iter().map(EventIterItem::Block);
        let iter2 = self.outofbands.into_iter().map(EventIterItem::OutOfBand);
        EventIter {
            items: iter1.chain(iter2).collect(),
        }
    }
}

impl<'a> clause::DocumentClauseList<'a> {
    pub fn into_tokens(self) -> EventIter<'a> {
        let mut iter = EventIter {
            items: VecDeque::new(),
        };
        for (idx, clause) in self.clauses.into_iter().enumerate() {
            if idx != 0 {
                iter.items
                    .push_back(EventIterItem::Event(pulldown_cmark::Event::Rule))
            }
            iter.items
                .push_back(EventIterItem::Event(pulldown_cmark::Event::Start(
                    pulldown_cmark::Tag::List(Some(clause.clause_idx as _)),
                )));
            iter.items
                .push_back(EventIterItem::Event(pulldown_cmark::Event::Start(
                    pulldown_cmark::Tag::Item,
                )));
            iter.items.extend(
                clause
                    .clause_contents
                    .into_iter()
                    .map(EventIterItem::Inline),
            );
            iter.items
                .push_back(EventIterItem::Event(pulldown_cmark::Event::End(
                    pulldown_cmark::Tag::Item,
                )));
            iter.items
                .push_back(EventIterItem::Event(pulldown_cmark::Event::End(
                    pulldown_cmark::Tag::List(Some(clause.clause_idx as _)),
                )));
            for (lang, lang_items) in clause.clause_translations {
                iter.items
                    .push_back(EventIterItem::Event(pulldown_cmark::Event::Start(
                        pulldown_cmark::Tag::Heading(3),
                    )));
                iter.items
                    .push_back(EventIterItem::Event(pulldown_cmark::Event::Text(lang)));
                iter.items
                    .push_back(EventIterItem::Event(pulldown_cmark::Event::End(
                        pulldown_cmark::Tag::Heading(3),
                    )));
                iter.items
                    .push_back(EventIterItem::Event(pulldown_cmark::Event::Start(
                        pulldown_cmark::Tag::Paragraph,
                    )));
                iter.items
                    .extend(lang_items.into_iter().map(|x| EventIterItem::Inline(x)));
                iter.items
                    .push_back(EventIterItem::Event(pulldown_cmark::Event::End(
                        pulldown_cmark::Tag::Paragraph,
                    )));
            }
        }
        iter
    }
}

pub struct EventIter<'a> {
    items: VecDeque<EventIterItem<'a>>,
}

pub trait ExtendFront<A> {
    fn extend_front<T: DoubleEndedIterator<Item = A>>(&mut self, iter: T);
}

impl<A> ExtendFront<A> for VecDeque<A> {
    fn extend_front<T: DoubleEndedIterator<Item = A>>(&mut self, mut iter: T) {
        while let Some(v) = iter.next_back() {
            self.push_front(v)
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = pulldown_cmark::Event<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        'restart: loop {
            let next_item = self.items.pop_front()?;
            match next_item {
                EventIterItem::Event(e) => return Some(e),
                EventIterItem::Inline(i) => match i {
                    ast::InlineNode::Surrounding { tag, contents } => {
                        match unconvert_tag(ConvertedTag::InBand(
                            ConvertedInbandTag::SurroundingInline(tag),
                        )) {
                            UnconvertedTag::SpanEvent(start, end) => {
                                self.items.push_front(EventIterItem::Event(end));
                                self.items
                                    .extend_front(contents.into_iter().map(EventIterItem::Inline));
                                return Some(start);
                            }
                            UnconvertedTag::TransparentEvent => {
                                self.items
                                    .extend_front(contents.into_iter().map(EventIterItem::Inline));
                                continue 'restart;
                            }
                            UnconvertedTag::SpanEventWithoutLength(_, _)
                            | UnconvertedTag::SingleEvent(_) => {
                                unreachable!();
                            }
                            UnconvertedTag::Custom(_) => {
                                //FIXME: todo!();
                                self.items
                                    .extend_front(contents.into_iter().map(EventIterItem::Inline));
                                continue 'restart;
                            }
                        };
                    }
                    ast::InlineNode::Content { tag } => {
                        match unconvert_tag(ConvertedTag::InBand(
                            ConvertedInbandTag::ContentInline(tag),
                        )) {
                            UnconvertedTag::SpanEvent(_, _) | UnconvertedTag::TransparentEvent => {
                                unreachable!()
                            }
                            UnconvertedTag::SpanEventWithoutLength(start, end) => {
                                self.items.push_front(EventIterItem::Event(end));
                                return Some(start);
                            }
                            UnconvertedTag::SingleEvent(e) => {
                                return Some(e);
                            }
                            UnconvertedTag::Custom(_) => {
                                todo!();
                            }
                        }
                    }
                },
                EventIterItem::Block(b) => match b {
                    ast::BlockNode::Container { tag, children } => {
                        match unconvert_tag(ConvertedTag::InBand(
                            ConvertedInbandTag::ContainerBlock(tag),
                        )) {
                            UnconvertedTag::SpanEvent(start, end) => {
                                self.items.push_front(EventIterItem::Event(end));
                                self.items
                                    .extend_front(children.into_iter().map(EventIterItem::Block));
                                return Some(start);
                            }
                            UnconvertedTag::TransparentEvent => {
                                self.items
                                    .extend_front(children.into_iter().map(EventIterItem::Block));
                                continue 'restart;
                            }
                            UnconvertedTag::SpanEventWithoutLength(_, _)
                            | UnconvertedTag::SingleEvent(_) => unreachable!(),
                            UnconvertedTag::Custom(_) => todo!(),
                        }
                    }
                    ast::BlockNode::Leaf { tag, contents } => {
                        match unconvert_tag(ConvertedTag::InBand(ConvertedInbandTag::LeafBlock(
                            tag,
                        ))) {
                            UnconvertedTag::SpanEvent(start, end) => {
                                self.items.push_front(EventIterItem::Event(end));
                                self.items
                                    .extend_front(contents.into_iter().map(EventIterItem::Inline));
                                return Some(start);
                            }
                            UnconvertedTag::TransparentEvent => {
                                self.items
                                    .extend_front(contents.into_iter().map(EventIterItem::Inline));
                                continue 'restart;
                            }
                            UnconvertedTag::SpanEventWithoutLength(_, _)
                            | UnconvertedTag::SingleEvent(_) => unreachable!(),
                            UnconvertedTag::Custom(_) => todo!(),
                        }
                    }
                    ast::BlockNode::Markup { tag } => {
                        match unconvert_tag(ConvertedTag::InBand(ConvertedInbandTag::MarkupBlock(
                            tag,
                        ))) {
                            UnconvertedTag::SpanEvent(_, _) | UnconvertedTag::TransparentEvent => {
                                unreachable!()
                            }
                            UnconvertedTag::SpanEventWithoutLength(start, end) => {
                                self.items.push_front(EventIterItem::Event(end));
                                return Some(start);
                            }
                            UnconvertedTag::SingleEvent(e) => {
                                return Some(e);
                            }
                            UnconvertedTag::Custom(_) => {
                                todo!();
                            }
                        }
                    }
                },
                EventIterItem::OutOfBand(o) => match o {
                    ast::OutOfBandNode::OutOfBandContainer { tag, children } => {
                        match unconvert_tag(ConvertedTag::OutOfBand(
                            ConvertedOutofbandTag::OutOfBandContainer(tag),
                        )) {
                            UnconvertedTag::SpanEvent(start, end) => {
                                self.items.push_front(EventIterItem::Event(end));
                                self.items
                                    .extend_front(children.into_iter().map(EventIterItem::Block));
                                return Some(start);
                            }
                            UnconvertedTag::TransparentEvent => {
                                self.items
                                    .extend_front(children.into_iter().map(EventIterItem::Block));
                                continue 'restart;
                            }
                            UnconvertedTag::SpanEventWithoutLength(_, _)
                            | UnconvertedTag::SingleEvent(_) => unreachable!(),
                            UnconvertedTag::Custom(_) => todo!(),
                        }
                    }
                    ast::OutOfBandNode::OutOfBandLeaf { tag, contents } => {
                        match unconvert_tag(ConvertedTag::OutOfBand(
                            ConvertedOutofbandTag::OutOfBandLeaf(tag),
                        )) {
                            UnconvertedTag::SpanEvent(start, end) => {
                                self.items.push_front(EventIterItem::Event(end));
                                self.items
                                    .extend_front(contents.into_iter().map(EventIterItem::Inline));
                                return Some(start);
                            }
                            UnconvertedTag::TransparentEvent => {
                                self.items
                                    .extend_front(contents.into_iter().map(EventIterItem::Inline));
                                continue 'restart;
                            }
                            UnconvertedTag::SpanEventWithoutLength(_, _)
                            | UnconvertedTag::SingleEvent(_) => unreachable!(),
                            UnconvertedTag::Custom(_) => todo!(),
                        }
                    }
                },
            }
        }
    }
}

#[derive(Clone)]
enum EventIterItem<'a> {
    Event(pulldown_cmark::Event<'a>),
    Inline(ast::InlineNode<'a>),
    Block(ast::BlockNode<'a>),
    OutOfBand(ast::OutOfBandNode<'a>),
}
