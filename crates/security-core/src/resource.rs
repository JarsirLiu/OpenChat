#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Read,
    Create,
    Update,
    Delete,
    Execute,
    Manage,
}

impl Action {
    pub fn is_read(self) -> bool {
        matches!(self, Self::Read)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Execute => "execute",
            Self::Manage => "manage",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Session,
    MediaObject,
    UserProviderApiKey,
    CustomModel,
    Catalog,
    System,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::MediaObject => "media object",
            Self::UserProviderApiKey => "user provider api key",
            Self::CustomModel => "custom model",
            Self::Catalog => "catalog",
            Self::System => "system resource",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceOwner {
    User(String),
    System,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResourceVisibility {
    Private,
    SharedRead,
    Public,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceDescriptor {
    pub kind: ResourceKind,
    pub owner: ResourceOwner,
    pub visibility: ResourceVisibility,
}

impl ResourceDescriptor {
    pub fn new(kind: ResourceKind, owner: ResourceOwner, visibility: ResourceVisibility) -> Self {
        Self {
            kind,
            owner,
            visibility,
        }
    }
}
