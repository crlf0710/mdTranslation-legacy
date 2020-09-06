use pulldown_cmark::CowStr;
pub use pulldown_cmark::{Alignment, CodeBlockKind, LinkType};
#[derive(Clone, PartialEq, Debug)]
pub enum ContainerBlockTag<'a> {
    BlockQuote,
    List(Option<u64>),
    ListItem,
    Table(Vec<Alignment>),
    TableHead,
    TableRow,
    Custom(CowStr<'a>),
}

#[derive(Clone, PartialEq, Debug)]
pub enum LeafBlockTag<'a> {
    Paragraph,
    Heading(u32),
    CodeBlock(CodeBlockKind<'a>),
    TableCell,
    Html,
    Custom(CowStr<'a>),
}

#[derive(Clone, PartialEq, Debug)]
pub enum MarkupBlockTag<'a> {
    Rule,
    Custom(CowStr<'a>),
}

#[derive(Clone, Debug)]
pub enum BlockNode<'a> {
    Container {
        tag: ContainerBlockTag<'a>,
        children: BlockNodeList<'a>,
    },
    Leaf {
        tag: LeafBlockTag<'a>,
        contents: Vec<InlineNode<'a>>,
    },
    Markup {
        tag: MarkupBlockTag<'a>,
    },
}

pub type BlockNodeList<'a> = Vec<BlockNode<'a>>;

#[derive(Clone, PartialEq, Debug)]
pub enum OutOfBandContainerTag<'a> {
    FootNoteDef(CowStr<'a>),
    Custom(CowStr<'a>),
}

#[derive(Clone, PartialEq, Debug)]
pub enum OutOfBandLeafTag<'a> {
    Custom(CowStr<'a>),
}

#[derive(Clone, Debug)]
pub enum OutOfBandNode<'a> {
    OutOfBandContainer {
        tag: OutOfBandContainerTag<'a>,
        children: BlockNodeList<'a>,
    },
    OutOfBandLeaf {
        tag: OutOfBandLeafTag<'a>,
        contents: InlineNodeList<'a>,
    },
}

pub type OutOfBandNodeList<'a> = Vec<OutOfBandNode<'a>>;

#[derive(Clone, PartialEq, Debug)]
pub enum SurroundingInlineTag<'a> {
    Emphasis,
    Strong,
    Strikethrough,
    Link(LinkType, CowStr<'a>, CowStr<'a>),
    Image(LinkType, CowStr<'a>, CowStr<'a>),
    Custom(CowStr<'a>),
}

#[derive(Clone, PartialEq, Debug)]
pub enum ContentInlineTag<'a> {
    Text(CowStr<'a>),
    Code(CowStr<'a>),
    RawHtml(CowStr<'a>),
    FootnoteRef(CowStr<'a>),
    TaskListMarker(bool),
    SoftBreak,
    HardBreak,
    Custom(CowStr<'a>),
}

#[derive(Clone, Debug)]
pub enum InlineNode<'a> {
    Surrounding {
        tag: SurroundingInlineTag<'a>,
        contents: InlineNodeList<'a>,
    },
    Content {
        tag: ContentInlineTag<'a>,
    },
}

pub type InlineNodeList<'a> = Vec<InlineNode<'a>>;

#[derive(Default, Clone, Debug)]
pub struct Document<'a> {
    pub(crate) blocks: BlockNodeList<'a>,
    pub(crate) outofbands: OutOfBandNodeList<'a>,
}
