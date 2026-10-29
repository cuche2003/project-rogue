use project_rogue_engine_core::{
    ActorDefinition, BehaviorKind, Command, CommandError, CommandResult, ContentPack, Engine,
    GameConfig, GameEvent, GridPosition, Health, ItemDefinition, ItemEffect, ItemSpawnDefinition,
    Player, Position,
};

fn content(items: Vec<ItemDefinition>, item_spawns: Vec<ItemSpawnDefinition>) -> ContentPack {
    ContentPack::new(
        vec![ActorDefinition {
            id: "core:player".to_owned(),
            name: "Player".to_owned(),
            glyph: '@',
            max_health: 10,
            attack_damage: 1,
            behavior: BehaviorKind::Passive,
        }],
        vec![],
        items,
        item_spawns,
    )
    .unwrap()
}

fn potion(id: &str, stack_limit: u32, power: u32) -> ItemDefinition {
    ItemDefinition {
        id: id.to_owned(),
        name: "Potion".to_owned(),
        glyph: '!',
        stack_limit,
        effect: ItemEffect::Heal,
        power,
    }
}

fn engine(items: Vec<ItemDefinition>, item_spawns: Vec<ItemSpawnDefinition>) -> Engine {
    Engine::with_content(
        GameConfig {
            seed: 0,
            map_width: 8,
            map_height: 8,
            floor_count: 1,
        },
        content(items, item_spawns),
    )
    .unwrap()
}

fn set_player_position(engine: &mut Engine, position: Position) {
    let player = engine
        .world()
        .iter_entities()
        .find(|entity| entity.get::<Player>().is_some())
        .unwrap()
        .id();
    engine
        .world_mut()
        .entity_mut(player)
        .insert(GridPosition(position));
}

#[test]
fn custom_item_power_changes_healing() {
    let mut engine = engine(
        vec![potion("mod:strong-potion", 1, 7)],
        vec![ItemSpawnDefinition {
            item_id: "mod:strong-potion".to_owned(),
            count: 1,
        }],
    );
    let position = engine.snapshot().items[0].position.unwrap();
    set_player_position(&mut engine, position);
    let player = engine
        .world()
        .iter_entities()
        .find(|entity| entity.get::<Player>().is_some())
        .unwrap()
        .id();
    engine
        .world_mut()
        .get_mut::<Health>(player)
        .unwrap()
        .current = 2;

    assert_eq!(engine.dispatch(Command::Pickup), CommandResult::Applied);
    let pickup = engine.drain_events();
    let [GameEvent::ItemPickedUp { item }] = pickup.as_slice() else {
        panic!("expected item pickup")
    };
    assert_eq!(engine.dispatch(Command::UseItem(0)), CommandResult::Applied);
    assert_eq!(engine.snapshot().player.health.current, 9);
    assert_eq!(
        engine.drain_events(),
        vec![
            GameEvent::HealthRestored {
                source: Some(project_rogue_engine_core::EntityId(0)),
                target: project_rogue_engine_core::EntityId(0),
                target_position: engine.player_position(),
                amount: 7,
                current_hp: 9,
            },
            GameEvent::ItemUsed {
                item: *item,
                item_id: "mod:strong-potion".to_owned(),
                item_name: "Potion".to_owned(),
                effect: ItemEffect::Heal,
                target: project_rogue_engine_core::EntityId(0),
                amount: 7,
            },
        ]
    );
}

#[test]
fn ground_items_have_deterministic_ids_and_positions() {
    let items = vec![potion("mod:potion", 5, 3)];
    let spawns = vec![ItemSpawnDefinition {
        item_id: "mod:potion".to_owned(),
        count: 3,
    }];
    let first = engine(items.clone(), spawns.clone()).snapshot().items;
    let second = engine(items, spawns).snapshot().items;
    assert_eq!(first, second);
    assert_eq!(
        first.iter().map(|item| item.id).collect::<Vec<_>>().len(),
        3
    );
    assert!(first.iter().all(|item| item.position.is_some()));
}

#[test]
fn stack_limit_creates_new_inventory_slot_after_stack_fills() {
    let mut engine = engine(
        vec![potion("mod:potion", 2, 3)],
        vec![ItemSpawnDefinition {
            item_id: "mod:potion".to_owned(),
            count: 3,
        }],
    );
    let positions: Vec<_> = engine
        .snapshot()
        .items
        .iter()
        .map(|item| item.position.unwrap())
        .collect();
    for position in positions {
        set_player_position(&mut engine, position);
        assert_eq!(engine.dispatch(Command::Pickup), CommandResult::Applied);
    }
    let inventory = engine.snapshot().inventory;
    assert_eq!(inventory.len(), 2);
    assert_eq!(inventory[0].quantity, 2);
    assert_eq!(inventory[1].quantity, 1);
}

#[test]
fn unexplored_ground_items_stay_hidden_until_visible() {
    let mut engine = Engine::with_content(
        GameConfig::default(),
        content(
            vec![potion("mod:potion", 1, 1)],
            vec![ItemSpawnDefinition {
                item_id: "mod:potion".to_owned(),
                count: 1,
            }],
        ),
    )
    .unwrap();
    assert!(engine.snapshot().items.is_empty());
    let positions: Vec<_> = engine
        .world()
        .resource::<project_rogue_engine_core::Map>()
        .0
        .tiles()
        .iter()
        .enumerate()
        .filter_map(|(index, tile)| {
            (*tile == project_rogue_engine_core::Tile::Floor).then_some(Position {
                x: (index % 40) as i32,
                y: (index / 40) as i32,
            })
        })
        .collect();
    for position in positions {
        set_player_position(&mut engine, position);
        engine.dispatch(Command::Wait);
        if !engine.snapshot().items.is_empty() {
            return;
        }
    }
    panic!("ground item never became visible");
}

#[test]
fn invalid_item_capacity_is_rejected_before_run_creation() {
    let result = Engine::with_content(
        GameConfig {
            map_width: 8,
            map_height: 8,
            ..GameConfig::default()
        },
        content(
            vec![potion("mod:potion", 999, 1)],
            vec![ItemSpawnDefinition {
                item_id: "mod:potion".to_owned(),
                count: 1_000,
            }],
        ),
    );
    assert!(matches!(
        result,
        Err(project_rogue_engine_core::EngineError::InsufficientItemTiles { .. })
    ));
}

#[test]
fn full_inventory_rejects_new_item_but_allows_existing_stack() {
    let items: Vec<_> = (0..10)
        .map(|index| potion(&format!("mod:potion-{index}"), 1, 1))
        .collect();
    let item_spawns: Vec<_> = (0..10)
        .map(|index| ItemSpawnDefinition {
            item_id: format!("mod:potion-{index}"),
            count: 1,
        })
        .collect();
    let mut engine = (0..100)
        .find_map(|seed| {
            let content = content(items.clone(), item_spawns.clone());
            Engine::with_content(
                GameConfig {
                    seed,
                    map_width: 8,
                    map_height: 8,
                    floor_count: 1,
                },
                content,
            )
            .ok()
            .filter(|engine| engine.snapshot().items.len() == 10)
        })
        .expect("8x8 seed with ten visible floor items");
    let positions: Vec<_> = engine
        .snapshot()
        .items
        .iter()
        .map(|item| item.position.unwrap())
        .collect();
    for position in positions.iter().take(9) {
        set_player_position(&mut engine, *position);
        assert_eq!(engine.dispatch(Command::Pickup), CommandResult::Applied);
    }
    set_player_position(&mut engine, positions[9]);
    assert_eq!(
        engine.dispatch(Command::Pickup),
        CommandResult::Rejected(CommandError::InventoryFull)
    );
}
