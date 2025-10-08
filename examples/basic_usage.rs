// Example: Basic usage of the notiq-core library
use std::fs;

use notiq_core::models::*;
use notiq_core::storage::*;
use chrono::Utc;

fn main() -> anyhow::Result<()> {
    let db_path = "basic_usage_notiq.db";
    fs::remove_file(db_path).ok(); // Clean up previous run

    println!("--- Basic Usage of notiq-core ---");

    // Initialize database
    let db = Database::new(db_path);
    let conn = db.create()?;
    println!("   ✓ Database created with schema version {}", db.get_schema_version(&conn)?);
    
    // ========== Create Notes ==========
    println!("\n2. Creating notes...");
    let mut project_note = Note::new("Project Planning".to_string());
    NoteRepository::create(&conn, &project_note)?;
    println!("   ✓ Created note: {}", project_note.title);
    
    let mut ideas_note = Note::new("Ideas".to_string());
    NoteRepository::create(&conn, &ideas_note)?;
    println!("   ✓ Created note: {}", ideas_note.title);
    
    // ========== Create Outline Structure ==========
    println!("\n3. Creating outline structure...");
    
    // Root level items
    let goals = OutlineNode::new(
        project_note.id.clone(),
        None,
        "Q4 Goals".to_string(),
        0,
    );
    NodeRepository::create(&conn, &goals)?;
    println!("   ✓ Created node: {}", goals.content);
    
    // Child items
    let goal1 = OutlineNode::new(
        project_note.id.clone(),
        Some(goals.id.clone()),
        "Launch new feature".to_string(),
        0,
    );
    NodeRepository::create(&conn, &goal1)?;
    
    let goal2 = OutlineNode::new(
        project_note.id.clone(),
        Some(goals.id.clone()),
        "Improve documentation".to_string(),
        1,
    );
    NodeRepository::create(&conn, &goal2)?;
    println!("   ✓ Created 2 child nodes");
    
    // ========== Create Tasks ==========
    println!("\n4. Creating tasks...");
    
    let task1 = OutlineNode::new_task(
        project_note.id.clone(),
        None,
        "Complete Phase 1 implementation".to_string(),
        1,
        Some(TaskPriority::High),
        None,
    );
    NodeRepository::create(&conn, &task1)?;
    println!("   ✓ Created high priority task");
    
    let task2 = OutlineNode::new_task(
        project_note.id.clone(),
        None,
        "Write documentation".to_string(),
        2,
        Some(TaskPriority::Medium),
        Some(Utc::now() + chrono::Duration::days(7)),
    );
    NodeRepository::create(&conn, &task2)?;
    println!("   ✓ Created task with due date");
    
    // ========== Complete a Task ==========
    println!("\n5. Completing a task...");
    let mut task = NodeRepository::get_by_id(&conn, &task1.id)?;
    task.toggle_task();
    NodeRepository::update(&conn, &task)?;
    
    // Log the task completion
    let log = TaskStatusLog::new(
        task.id.clone(),
        TaskStatus::Completed,
        Some("false".to_string()),
        Some("true".to_string()),
    );
    TaskLogRepository::create(&conn, &log)?;
    println!("   ✓ Task completed and logged");
    
    // ========== Add Tags ==========
    println!("\n6. Adding tags...");
    
    let work_tag = TagRepository::get_or_create(&conn, "work", Some("#3498db".to_string()))?;
    let planning_tag = TagRepository::get_or_create(&conn, "planning", Some("#e74c3c".to_string()))?;
    
    TagRepository::add_to_node(&conn, &goals.id, work_tag.id.unwrap())?;
    TagRepository::add_to_node(&conn, &goals.id, planning_tag.id.unwrap())?;
    println!("   ✓ Added 2 tags to node");
    
    // ========== Create Links ==========
    println!("\n7. Creating links...");
    
    let link = Link::new_wiki_link(
        project_note.id.clone(),
        Some(goals.id.clone()),
        ideas_note.id.clone(),
        Some("Ideas".to_string()),
    );
    LinkRepository::create(&conn, &link)?;
    println!("   ✓ Created wiki link between notes");
    
    // ========== Add to Favorites ==========
    println!("\n8. Adding to favorites...");
    
    let position = FavoriteRepository::get_next_position(&conn)?;
    let favorite = Favorite::new(project_note.id.clone(), position);
    FavoriteRepository::create(&conn, &favorite)?;
    println!("   ✓ Added note to favorites");
    
    // ========== Create Daily Note ==========
    println!("\n9. Creating daily note...");
    
    let today = chrono::Local::now().date_naive();
    let daily = DailyNoteRepository::get_or_create(&conn, today, project_note.id.clone())?;
    println!("   ✓ Created daily note for {}", daily.date_string());
    
    // ========== Query and Display ==========
    println!("\n10. Querying data...");
    
    let all_notes = NoteRepository::get_all(&conn)?;
    println!("   • Total notes: {}", all_notes.len());
    
    let project_nodes = NodeRepository::get_by_note_id(&conn, &project_note.id)?;
    println!("   • Nodes in project: {}", project_nodes.len());
    
    let open_tasks = NodeRepository::get_tasks(&conn, Some(false))?;
    println!("   • Open tasks: {}", open_tasks.len());
    
    let backlinks = LinkRepository::get_backlinks(&conn, &ideas_note.id)?;
    println!("   • Backlinks to Ideas note: {}", backlinks.len());
    
    let tag_counts = TagRepository::get_usage_counts(&conn)?;
    for (tag, count) in tag_counts {
        println!("   • Tag '{}': {} uses", tag.name, count);
    }
    
    // ========== Full-Text Search ==========
    println!("\n11. Testing full-text search...");
    
    let search_results = NodeRepository::search(&conn, "feature")?;
    println!("   • Search for 'feature': {} results", search_results.len());
    for result in search_results {
        println!("     - {}", result.content);
    }
    
    // ========== Statistics ==========
    println!("\n12. Database statistics:");
    println!("   • Total notes: {}", NoteRepository::count(&conn)?);
    println!("   • Schema version: {}", db.get_schema_version(&conn)?);
    
    // ========== Cleanup ==========
    println!("\n13. Creating backup...");
    db.backup("example-backup.db")?;
    println!("   ✓ Backup created");
    
    println!("\n✅ Example completed successfully!");
    println!("\nDatabase file: example.db");
    println!("Backup file: example-backup.db");
    println!("\nYou can inspect the database with: sqlite3 example.db");
    
    Ok(())
}

