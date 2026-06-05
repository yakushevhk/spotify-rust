use std::collections::HashMap;

use rspotify::model::Id;

use crate::state::{Playlist, PlaylistFolder, PlaylistFolderItem, PlaylistFolderNode};

/// Structurize a flat input playlist according to the playlist folder nodes
pub fn structurize(
    playlists: Vec<Playlist>,
    nodes: &[PlaylistFolderNode],
) -> Vec<PlaylistFolderItem> {
    let mut playlist_folders = Vec::new();

    let mut playlists = playlists
        .into_iter()
        .map(|p| (p.id.id().to_string(), p))
        .collect::<HashMap<_, _>>();

    // Construct playlist folders with relevant playlists
    add_playlist_folders(nodes, &mut playlists, &mut 0, &mut playlist_folders, 0);

    // Remaining playlists that don't belong to any folders are added as root playlists
    for (_, mut p) in playlists {
        p.current_folder_id = 0;
        playlist_folders.push(PlaylistFolderItem::Playlist(p));
    }
    playlist_folders
}

const MAX_FOLDER_DEPTH: usize = 20;

fn add_playlist_folders(
    nodes: &[PlaylistFolderNode],
    playlists: &mut HashMap<String, Playlist>,
    folder_id: &mut usize,
    acc: &mut Vec<PlaylistFolderItem>,
    depth: usize,
) {
    if depth >= MAX_FOLDER_DEPTH {
        tracing::warn!("Playlist folder nesting exceeds {MAX_FOLDER_DEPTH} levels, stopping recursion to prevent stack overflow");
        return;
    }
    let current_folder_id = *folder_id;
    for f in nodes {
        if let Some((_, id)) = f.uri.rsplit_once(':') {
            // node_type is a string from the external spotify-folders JSON.
            // "folder" is the only non-track node type defined by that tool.
            if f.node_type == "folder" {
                *folder_id += 1;
                let name = f
                    .name
                    .clone()
                    .unwrap_or(format!("folder_{current_folder_id}"));
                // Folder node
                acc.push(PlaylistFolderItem::Folder(PlaylistFolder {
                    name: name.clone(),
                    current_id: current_folder_id,
                    target_id: *folder_id,
                }));
                // Up node
                acc.push(PlaylistFolderItem::Folder(PlaylistFolder {
                    name: format!("← {name}"),
                    current_id: *folder_id,
                    target_id: current_folder_id,
                }));
                add_playlist_folders(&f.children, playlists, folder_id, acc, depth + 1);
            } else if let Some(mut p) = playlists.remove(id) {
                p.current_folder_id = current_folder_id;
                acc.push(PlaylistFolderItem::Playlist(p));
            }
        }
    }
}
