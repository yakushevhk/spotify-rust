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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Playlist, PlaylistFolderNode, PlaylistId, UserId};

    fn create_test_playlist(id: &str) -> Playlist {
        Playlist {
            id: PlaylistId::from_id(id).unwrap().into_static(),
            collaborative: false,
            name: format!("Playlist {}", id),
            owner: ("Owner".to_string(), UserId::from_id("spotify").unwrap().into_static()),
            desc: "Description".to_string(),
            current_folder_id: 0,
            snapshot_id: "snapshot".to_string(),
            cover_url: None,
            image_path: None,
        }
    }

    fn create_folder_node(name: &str, children: Vec<PlaylistFolderNode>) -> PlaylistFolderNode {
        PlaylistFolderNode {
            name: Some(name.to_string()),
            node_type: "folder".to_string(),
            uri: format!("folder:{}", name.to_lowercase()),
            children,
        }
    }

    fn create_playlist_node(playlist_id: &str) -> PlaylistFolderNode {
        PlaylistFolderNode {
            name: Some(format!("Playlist {}", playlist_id)),
            node_type: "playlist".to_string(),
            uri: format!("spotify:playlist:{}", playlist_id),
            children: vec![],
        }
    }

    /// Test structurize with flat playlist list (no folders)
    #[test]
    fn test_structurize_flat_list() {
        let playlists = vec![
            create_test_playlist("3n3Ppam7vgaVa1iaRUc9Lp"),
            create_test_playlist("4uLU6hMCjMI75M1A2tKUQC"),
            create_test_playlist("p3"),
        ];
        
        let result = structurize(playlists, &[]);
        
        // All playlists should be at root level
        assert_eq!(result.len(), 3);
        assert!(matches!(result[0], PlaylistFolderItem::Playlist(_)));
        assert!(matches!(result[1], PlaylistFolderItem::Playlist(_)));
        assert!(matches!(result[2], PlaylistFolderItem::Playlist(_)));
    }

    /// Test structurize with single folder
    #[test]
    fn test_structurize_single_folder() {
        let playlists = vec![
            create_test_playlist("3n3Ppam7vgaVa1iaRUc9Lp"),
        ];
        
        let nodes = vec![
            create_folder_node("My Folder", vec![
                create_playlist_node("3n3Ppam7vgaVa1iaRUc9Lp"),
            ]),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // Should have: folder, up-link, playlist
        assert_eq!(result.len(), 3);
        assert!(matches!(result[0], PlaylistFolderItem::Folder(_)));
        assert!(matches!(result[1], PlaylistFolderItem::Folder(_))); // Up-link
        assert!(matches!(result[2], PlaylistFolderItem::Playlist(_)));
    }

    /// Test structurize with deep nesting
    #[test]
    fn test_structurize_deep_nesting() {
        let playlists = vec![
            create_test_playlist("3n3Ppam7vgaVa1iaRUc9Lp"),
        ];
        
        // Create nested structure: Root -> Child -> Grandchild -> Playlist
        let nodes = vec![
            create_folder_node("Root", vec![
                create_folder_node("Child", vec![
                    create_folder_node("Grandchild", vec![
                        create_playlist_node("3n3Ppam7vgaVa1iaRUc9Lp"),
                    ]),
                ]),
            ]),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // Should have multiple folders and up-links
        assert!(result.len() > 1);
        
        // Count folders (each level adds a folder and up-link)
        let folder_count = result.iter()
            .filter(|item| matches!(item, PlaylistFolderItem::Folder(_)))
            .count();
        assert!(folder_count >= 6); // 3 folders + 3 up-links
    }

    /// Test structurize with multiple folders at same level
    #[test]
    fn test_structurize_multiple_folders() {
        let playlists = vec![
            create_test_playlist("3n3Ppam7vgaVa1iaRUc9Lp"),
            create_test_playlist("4uLU6hMCjMI75M1A2tKUQC"),
        ];
        
        let nodes = vec![
            create_folder_node("Folder A", vec![
                create_playlist_node("3n3Ppam7vgaVa1iaRUc9Lp"),
            ]),
            create_folder_node("Folder B", vec![
                create_playlist_node("4uLU6hMCjMI75M1A2tKUQC"),
            ]),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // Should have: Folder A, up-link, playlist, Folder B, up-link, playlist
        assert_eq!(result.len(), 6);
    }

    /// Test structurize with missing playlist (not in nodes)
    #[test]
    fn test_structurize_missing_playlist() {
        let playlists = vec![
            create_test_playlist("3n3Ppam7vgaVa1iaRUc9Lp"),
            create_test_playlist("4uLU6hMCjMI75M1A2tKUQC"),
        ];
        
        let nodes = vec![
            create_folder_node("My Folder", vec![
                create_playlist_node("3n3Ppam7vgaVa1iaRUc9Lp"),
            ]),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // Should have: folder, up-link, playlist (in folder), playlist (not in folder)
        assert_eq!(result.len(), 4);
        
        // The "4uLU6hMCjMI75M1A2tKUQC" playlist should be at root level
        let root_playlists: Vec<_> = result.iter()
            .filter(|item| matches!(item, PlaylistFolderItem::Playlist(_)))
            .collect();
        assert_eq!(root_playlists.len(), 2);
    }

    /// Test structurize with empty playlist list
    #[test]
    fn test_structurize_empty_playlists() {
        let playlists: Vec<Playlist> = vec![];
        
        let nodes = vec![
            create_folder_node("Empty Folder", vec![]),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // Should have folder and up-link, but no playlists
        assert_eq!(result.len(), 2);
        assert!(matches!(result[0], PlaylistFolderItem::Folder(_)));
        assert!(matches!(result[1], PlaylistFolderItem::Folder(_)));
    }

    /// Test structurize with empty nodes
    #[test]
    fn test_structurize_empty_nodes() {
        let playlists = vec![
            create_test_playlist("3n3Ppam7vgaVa1iaRUc9Lp"),
        ];
        
        let result = structurize(playlists, &[]);
        
        // Playlist should be at root level
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], PlaylistFolderItem::Playlist(_)));
    }

    /// Test folder ID assignment
    #[test]
    fn test_structurize_folder_id_assignment() {
        let playlists = vec![
            create_test_playlist("3n3Ppam7vgaVa1iaRUc9Lp"),
        ];
        
        let nodes = vec![
            create_folder_node("Folder", vec![
                create_playlist_node("3n3Ppam7vgaVa1iaRUc9Lp"),
            ]),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // Find the playlist in the result
        let playlist_item = result.iter()
            .find(|item| matches!(item, PlaylistFolderItem::Playlist(_)))
            .unwrap();
        
        if let PlaylistFolderItem::Playlist(p) = playlist_item {
            // Playlist should have folder_id > 0 (inside a folder)
            assert!(p.current_folder_id > 0);
        }
    }

    /// Test folder names are preserved
    #[test]
    fn test_structurize_folder_names() {
        let playlists = vec![];
        
        let nodes = vec![
            create_folder_node("Custom Folder Name", vec![]),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // First item should be the folder with correct name
        if let PlaylistFolderItem::Folder(f) = &result[0] {
            assert_eq!(f.name, "Custom Folder Name");
        } else {
            panic!("Expected folder");
        }
    }

    /// Test up-link folder name format
    #[test]
    fn test_structurize_up_link_name() {
        let playlists = vec![];
        
        let nodes = vec![
            create_folder_node("My Folder", vec![]),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // Second item should be the up-link
        if let PlaylistFolderItem::Folder(f) = &result[1] {
            assert!(f.name.starts_with("← "));
            assert!(f.name.contains("My Folder"));
        } else {
            panic!("Expected up-link folder");
        }
    }

    /// Test folder without name gets generated name
    #[test]
    fn test_structurize_folder_without_name() {
        let playlists = vec![];
        
        let node = PlaylistFolderNode {
            name: None,
            node_type: "folder".to_string(),
            uri: "folder:test".to_string(),
            children: vec![],
        };
        
        let result = structurize(playlists, &[node]);
        
        // Folder should have a generated name
        if let PlaylistFolderItem::Folder(f) = &result[0] {
            assert!(f.name.starts_with("folder_"));
        } else {
            panic!("Expected folder");
        }
    }

    /// Test MAX_FOLDER_DEPTH protection
    #[test]
    fn test_structurize_max_depth_protection() {
        let playlists = vec![create_test_playlist("3n3Ppam7vgaVa1iaRUc9Lp")];
        
        // Create a deeply nested structure that exceeds MAX_FOLDER_DEPTH
        fn create_deep_nesting(depth: usize) -> Vec<PlaylistFolderNode> {
            if depth == 0 {
                vec![create_playlist_node("3n3Ppam7vgaVa1iaRUc9Lp")]
            } else {
                vec![create_folder_node(
                    &format!("Level {}", depth),
                    create_deep_nesting(depth - 1),
                )]
            }
        }
        
        let nodes = create_deep_nesting(MAX_FOLDER_DEPTH + 5);
        
        // Should not panic or overflow stack
        let result = structurize(playlists, &nodes);
        assert!(!result.is_empty());
    }

    /// Test node with invalid URI format
    #[test]
    fn test_structurize_invalid_uri() {
        let playlists = vec![];
        
        let node = PlaylistFolderNode {
            name: Some("Invalid".to_string()),
            node_type: "playlist".to_string(),
            uri: "invalid-uri-without-colon".to_string(),
            children: vec![],
        };
        
        let result = structurize(playlists, &[node]);
        
        // Should handle invalid URI gracefully
        assert!(result.is_empty());
    }

    /// Test mixed playlist and folder nodes
    #[test]
    fn test_structurize_mixed_nodes() {
        let playlists = vec![
            create_test_playlist("3n3Ppam7vgaVa1iaRUc9Lp"),
            create_test_playlist("1301WleyT98MSxVHPZCA6M"),
        ];
        
        let nodes = vec![
            create_folder_node("My Folder", vec![
                create_playlist_node("3n3Ppam7vgaVa1iaRUc9Lp"),
            ]),
            create_playlist_node("1301WleyT98MSxVHPZCA6M"),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // Should have: folder, up-link, playlist (in folder), playlist (at root)
        assert_eq!(result.len(), 4);
    }

    /// Test folder target_id and current_id relationship
    #[test]
    fn test_structurize_folder_ids() {
        let playlists = vec![];
        
        let nodes = vec![
            create_folder_node("My Folder", vec![]),
        ];
        
        let result = structurize(playlists, &nodes);
        
        // First folder should have target_id pointing to up-link
        if let PlaylistFolderItem::Folder(f) = &result[0] {
            // The up-link should point back to the folder
            if let PlaylistFolderItem::Folder(up) = &result[1] {
                assert_eq!(f.target_id, up.current_id);
                assert_eq!(up.target_id, f.current_id);
            } else {
                panic!("Expected up-link folder");
            }
        } else {
            panic!("Expected folder");
        }
    }
}
