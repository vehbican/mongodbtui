use futures::stream::TryStreamExt;
use mongodb::{
    Client,
    bson::{Document, doc},
};
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader, Write},
};
use tokio::fs::{self};

#[derive(Clone, Debug)]
pub struct ExportProgress {
    pub label: String,
    pub completed: u64,
    pub total: u64,
    pub collection_completed: usize,
    pub collection_total: usize,
}

pub type ExportProgressSender = tokio::sync::mpsc::UnboundedSender<ExportProgress>;

pub async fn export_collection_with_progress(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    file_path: &str,
    progress: Option<&ExportProgressSender>,
    offset: u64,
    total: u64,
    collection_completed: usize,
    collection_total: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = client.database(db_name);
    let collection = db.collection::<Document>(collection_name);

    // count_documents can scan the whole collection and leave the UI at 0/0
    // for a long time. MongoDB's estimated count is metadata-backed and is
    // sufficient for a responsive export progress indicator.
    let document_total = collection.estimated_document_count().await?;
    let total = if total == 0 { document_total } else { total };
    let _ = progress.map(|sender| {
        sender.send(ExportProgress {
            label: collection_name.to_string(),
            completed: offset,
            total,
            collection_completed,
            collection_total,
        })
    });
    let mut cursor = collection.find(doc! {}).await?;

    let path = std::path::Path::new(file_path);

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut file = File::create(path)?;

    let mut completed = 0;
    while let Some(doc) = cursor.try_next().await? {
        let json_str = serde_json::to_string(&doc)?;
        writeln!(file, "{}", json_str)?;
        completed += 1;
        let _ = progress.map(|sender| {
            sender.send(ExportProgress {
                label: collection_name.to_string(),
                completed: offset + completed,
                total,
                collection_completed,
                collection_total,
            })
        });
    }

    Ok(())
}

pub async fn export_database_with_progress(
    client: &Client,
    db_name: &str,
    folder_path: &str,
    progress: Option<ExportProgressSender>,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::fs::create_dir_all(folder_path).await?;

    let db = client.database(db_name);
    let collections = db.list_collection_names().await?;
    let collection_total = collections.len();
    for (index, collection_name) in collections.into_iter().enumerate() {
        let file_path = format!("{}/{}.json", folder_path, collection_name);
        let path = std::path::Path::new(&file_path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        export_collection_with_progress(
            client,
            db_name,
            &collection_name,
            &file_path,
            progress.as_ref(),
            0,
            0,
            index + 1,
            collection_total,
        )
        .await?;
    }

    Ok(())
}
pub async fn import_collection_with_progress(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    file_path: &str,
    progress: Option<&ExportProgressSender>,
    collection_completed: usize,
    collection_total: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = client.database(db_name);
    let collection = db.collection::<Document>(collection_name);

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut docs = vec![];

    for line in reader.lines() {
        let line = line?;
        let doc: Document = serde_json::from_str(&line)?;
        docs.push(doc);
    }

    let total = docs.len() as u64;
    let _ = progress.as_ref().map(|sender| {
        sender.send(ExportProgress {
            label: collection_name.to_string(),
            completed: 0,
            total,
            collection_completed,
            collection_total,
        })
    });

    let mut completed = 0;
    while !docs.is_empty() {
        let batch_size = docs.len().min(100);
        let batch: Vec<_> = docs.drain(..batch_size).collect();
        collection.insert_many(batch).await?;
        completed += batch_size as u64;
        let _ = progress.as_ref().map(|sender| {
            sender.send(ExportProgress {
                label: collection_name.to_string(),
                completed,
                total,
                collection_completed,
                collection_total,
            })
        });
    }

    Ok(())
}
pub async fn import_database_with_progress(
    client: &Client,
    db_name: &str,
    folder_path: &str,
    progress: Option<ExportProgressSender>,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = fs::read_dir(folder_path).await?;

    tokio::pin!(entries);

    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(OsStr::to_str) {
                if file_name.ends_with(".json") {
                    files.push((file_name.trim_end_matches(".json").to_string(), path));
                }
            }
        }
    }

    let collection_total = files.len();
    for (index, (collection_name, path)) in files.into_iter().enumerate() {
        import_collection_with_progress(
            client,
            db_name,
            &collection_name,
            path.to_str().unwrap(),
            progress.as_ref(),
            index + 1,
            collection_total,
        )
        .await?;
    }

    Ok(())
}
