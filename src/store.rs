//! SceneShop — stash (warehouse, 5×3 × tabs I/II/III) on the left, STORE (5×3) on the right, BUY, description
//! box with price, GET MORE. Drag store→stash buys, stash→store sells. Stock is rolled from the ShopRefresh.xml
//! pool (assets/data/shop_pool.ron), filtered by the player's progress, and re-rolled after each victory.
use crate::{anim::Sheets, common::*, data::GameData, save::{Save, SHOP}, ui::*, AppState, Selected};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_resource::<DragState>().init_resource::<SlotEvents>().init_resource::<Tab>()
        .add_systems(OnEnter(AppState::Store), enter)
        .add_systems(Update, (drag_system, logic).chain().run_if(in_state(AppState::Store)))
        .add_systems(OnExit(AppState::Store), despawn_screen);
}

const STASH_C: Vec2 = Vec2::new(-242.0, 20.0);
const SHOP_C: Vec2 = Vec2::new(242.0, 20.0);

pub fn refresh_shop(data: &GameData, save: &mut Save) {
    let node = &data.world.nodes[(save.unlocked as usize).min(data.world.nodes.len() - 1)];
    let cap = node.max_level.max(4) + 2;
    let pool: Vec<i32> = data.shop_pool.iter().copied().filter(|&i| data.item(i).map(|e| e.level <= cap && e.price > 0).unwrap_or(false)).collect();
    save.shop = (0..SHOP).map(|_| if pool.is_empty() { None } else { Some(pool[fastrand::usize(..pool.len())]) }).collect();
}

/// SceneShop sell: (price + upgrade costs) × (100 + merchant bonus)/100 / 10. Items here carry no upgrade level, bonus 0.
pub fn sell_price(data: &GameData, id: i32) -> i64 { (data.item(id).map(|e| e.price).unwrap_or(0) as i64 / 10).max(1) }

#[derive(Resource, Default)]
struct StoreSel(Option<Slot>);

fn enter(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, sheets: Res<Sheets>, mut save: ResMut<Save>, mut tab: ResMut<Tab>, sel: Res<Selected>) {
    tab.0 = 0;
    if save.shop.iter().all(|s| s.is_none()) { refresh_shop(&data, &mut save); }
    image(&mut cmd, &a, "ui/team_bg.png", Vec2::ZERO, Some(SCREEN), 0.0);
    portrait_strip(&mut cmd, &a, &data, &sheets, &save, sel.0, &save.party.clone());
    back_and_gold(&mut cmd, &a, &f, &save, AppState::Tavern);
    image(&mut cmd, &a, "ui/warehouse.png", STASH_C, None, 2.0);
    image(&mut cmd, &a, "ui/store.png", SHOP_C, None, 2.0);
    for i in 0..15 {
        slot_tile(&mut cmd, &a, Slot::Stash(i), grid_pos(STASH_C, i), "ui/item.png", 3.0);
        slot_tile(&mut cmd, &a, Slot::Shop(i), grid_pos(SHOP_C, i), "ui/item.png", 3.0);
    }
    tabs(&mut cmd, &a, &f, STASH_C, 0, 1, save.bag_tabs as usize);
    desc_box(&mut cmd, &a, &f, Vec2::new(-110.0, -245.0));
    // class-chip key under the store grid (own design; keeps the desc box free)
    let lg = text(&mut cmd, &f, CLASS_LEGEND, Vec2::new(242.0, -155.0), 11.0, Color::srgb(1.0, 0.92, 0.65), 6.0);
    cmd.entity(lg).insert(FitBox { max_w: 420.0, max_h: 28.0, base: 11.0, min: 8.0 });
    button(&mut cmd, &a, "ui/button_buy.png", Some("ui/button_buy_0.png"), Vec2::new(425.0, -200.0), Vec2::new(105.0, 45.0), 5.0, Act::Custom(2, 0));
    button(&mut cmd, &a, "ui/button_sell.png", Some("ui/button_sell_0.png"), Vec2::new(315.0, -200.0), Vec2::new(105.0, 45.0), 5.0, Act::Custom(3, 0));
    button(&mut cmd, &a, "ui/kengqian.png", Some("ui/kengqian_0.png"), Vec2::new(398.0, 174.0), Vec2::new(112.0, 40.0), 5.0, Act::Custom(4, 0));
    let rc = data.prices(save.unlocked).map(|p| p.refresh_shop).unwrap_or(70);
    text(&mut cmd, &f, &format!("{rc}"), Vec2::new(322.0, 174.0), 18.0, Color::srgb(1.0, 0.85, 0.3), 6.0);
    button(&mut cmd, &a, "ui/GETMORE.png", Some("ui/GETMORE_0.png"), Vec2::new(390.0, -275.0), Vec2::new(186.0, 76.0), 5.0, Act::Stub("GET MORE — more gold & items (not available in this remake)"));
    cmd.insert_resource(StoreSel(None));
    cmd.insert_resource(Rebuild(true));
}

#[derive(Resource)]
pub struct Rebuild(pub bool);

#[allow(clippy::too_many_arguments)]
fn logic(mut cmd: Commands, a: Res<AssetServer>, f: Res<Fonts>, data: Res<GameData>, mut save: ResMut<Save>, mut tab: ResMut<Tab>, mut sel: ResMut<Selected>,
         mut ev: ResMut<SlotEvents>, mut clicked: ResMut<Clicked>, mut ssel: ResMut<StoreSel>, mut rb: ResMut<Rebuild>,
         (slots, icons, marks): (Query<(&ItemSlot, &Transform)>, Query<Entity, With<ItemIcon>>, Query<Entity, With<ClassMark>>), (mut sfx, mut toast): (ResMut<crate::audio::Sfx>, ResMut<Toast>),
         (mut qt, mut qb): (Query<&mut Text2d, (With<DescTitle>, Without<DescBody>)>, Query<&mut Text2d, (With<DescBody>, Without<DescTitle>)>),
         (k, mut next, mut pend): (Res<ButtonInput<KeyCode>>, ResMut<NextState<AppState>>, Local<Option<(u32, i32)>>), (hl, tabq): (Query<Entity, With<Highlight>>, Query<Entity, With<TabBtn>>)) {
    if k.just_pressed(KeyCode::Escape) { next.set(AppState::Tavern); }
    let buy = |save: &mut Save, from: usize, to: Option<usize>, sfx: &mut crate::audio::Sfx, toast: &mut Toast| {
        let Some(id) = save.shop[from] else { return };
        let price = data.item(id).map(|e| e.price).unwrap_or(0) as i64;
        let Some(to) = to.filter(|&t| save.stash[t].is_none()).or_else(|| save.first_free_stash()) else { toast.0 = Some(("Stash is full".into(), 2.0)); return };
        if save.money() < price { toast.0 = Some(("Not enough gold".into(), 2.0)); sfx.play("sfx_wrong"); return; }
        save.add_money(-price); save.stash[to] = Some(id); save.shop[from] = None; sfx.play("sfx_buy"); save.write();
    };
    let sell = |save: &mut Save, i: usize, sfx: &mut crate::audio::Sfx, toast: &mut Toast| {
        let Some(id) = save.stash[i] else { return };
        let p = sell_price(&data, id);
        save.stash[i] = None; save.add_money(p); sfx.play("sfx_buy"); toast.0 = Some((format!("+{p}"), 1.2)); save.write();
    };
    for act in clicked.0.drain(..) {
        match act {
            // locked bag tab: first click asks, second click within the toast pays (original: confirm dialog, 50,000 gold)
            Act::Custom(1, t) if (t as u8) >= save.bag_tabs => {
                if (t as u8) > save.bag_tabs { toast.0 = Some(("Unlock the previous tab first".into(), 2.0)); continue; }
                if *pend == Some((1, t)) && toast.0.is_some() {
                    if save.money() < crate::save::UNLOCK_COST { toast.0 = Some(("Not enough gold".into(), 2.0)); sfx.play("sfx_wrong"); }
                    else { save.add_money(-crate::save::UNLOCK_COST); save.bag_tabs += 1; save.write(); sfx.play("sfx_buy"); toast.0 = Some(("Tab unlocked".into(), 1.5));
                        tab.0 = t as usize; rb.0 = true; for e in tabq.iter() { cmd.entity(e).despawn(); } tabs(&mut cmd, &a, &f, STASH_C, t as usize, 1, save.bag_tabs as usize); }
                    *pend = None;
                } else { *pend = Some((1, t)); toast.0 = Some((format!("Unlock this tab for {} gold? Click again to confirm", crate::save::UNLOCK_COST), 3.0)); sfx.play("sfx_select"); }
            }
            Act::Custom(1, t) => { tab.0 = t as usize; rb.0 = true; for e in tabq.iter() { cmd.entity(e).despawn(); } tabs(&mut cmd, &a, &f, STASH_C, t as usize, 1, save.bag_tabs as usize); }
            Act::Custom(2, _) => if let Some(Slot::Shop(i)) = ssel.0 { buy(&mut save, i, None, &mut sfx, &mut toast); ssel.0 = None; rb.0 = true; },
            Act::Custom(3, _) => if let Some(Slot::Stash(i)) = ssel.0 { sell(&mut save, i, &mut sfx, &mut toast); ssel.0 = None; rb.0 = true; },
            // REFRESH (kengqian): re-roll the stock for MoneyConfig.xml RefreshShop gold
            Act::Custom(4, _) => {
                let c = data.prices(save.unlocked).map(|p| p.refresh_shop).unwrap_or(70) as i64;
                if save.money() < c { toast.0 = Some(("Not enough gold".into(), 2.0)); sfx.play("sfx_wrong"); }
                else { save.add_money(-c); refresh_shop(&data, &mut save); save.write(); sfx.play("sfx_buy"); toast.0 = Some((format!("Store refreshed (-{c})"), 1.5)); ssel.0 = None; rb.0 = true; }
            }
            Act::Custom(PORTRAIT_ACT, r) => { sel.0 = r as usize; }
            _ => {}
        }
    }
    for (from, to) in std::mem::take(&mut ev.drops) {
        match (from, to) {
            (Slot::Shop(i), Slot::Stash(j)) => buy(&mut save, i, Some(j), &mut sfx, &mut toast),
            (Slot::Stash(i), Slot::Shop(_)) => sell(&mut save, i, &mut sfx, &mut toast),
            (Slot::Stash(i), Slot::Stash(j)) => { save.stash.swap(i, j); save.write(); }
            _ => {}
        }
        rb.0 = true;
    }
    for c in std::mem::take(&mut ev.clicks) {
        ssel.0 = Some(c);
        let id = slot_item(&save, sel.0, c);
        let (t, mut b) = id.map(|i| item_desc(&data, i)).unwrap_or_default();
        if let (Slot::Stash(_), Some(i)) = (c, id) { b += &format!("  (sell for {})", sell_price(&data, i)); }
        set_desc(t, b, &mut qt, &mut qb);
        rb.0 = true;
    }
    if rb.0 {
        rb.0 = false;
        refresh_icons(&mut cmd, &a, &f, &data, &save, sel.0, tab.0, &slots, &icons, &marks, true);
        for e in hl.iter() { cmd.entity(e).despawn(); }
        if let Some(s) = ssel.0 {
            let p = match s { Slot::Shop(i) => Some(grid_pos(SHOP_C, i)), Slot::Stash(i) if i / 15 == tab.0 => Some(grid_pos(STASH_C, i % 15)), _ => None };
            if let Some(p) = p { let e = image(&mut cmd, &a, "ui/item_selected.png", p, Some(Vec2::splat(84.0)), 4.5); cmd.entity(e).insert(Highlight); }
        }
    }
}

#[derive(Component)]
pub struct Highlight;

