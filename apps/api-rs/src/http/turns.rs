use serde::Serialize;

#[derive(Serialize)]
pub struct TurnInterruptAcceptedDto {
    pub ok: bool,
}
