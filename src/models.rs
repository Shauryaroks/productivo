use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub notes: String,
    pub priority: u8, // 0 low, 1 med, 2 high
    pub due_date: Option<NaiveDate>,
    pub project: Option<String>,
    pub tags: String, // csv
    pub parent_id: Option<i64>,
    pub recur_rule: Option<String>,
    pub done_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Habit {
    pub id: i64,
    pub name: String,
    pub position: i64,
    pub archived: bool,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: i64,
    pub title: String,
    pub date: NaiveDate,
    pub time: Option<String>,
    pub category: String,
    pub color: String,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct Idea {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub status: String, // spark|brewing|active|shipped|dropped
    pub created_at: String,
}
