/// Top-level game state — corresponds to WL_PLAY.C / WL_GAME.C.
///
/// Drives the main game loop: ticking all actors, updating doors,
/// checking win/lose conditions, and orchestrating drawing.
use std::path::Path;

use crate::assets::graphics::GraphicsCache;
use crate::engine::renderer::{Renderer, View};
use crate::game::{actor::ActorList, door::DoorList, map::GameMap, player::Player};
use crate::input::handler::InputHandler;

#[derive(Debug)]
pub enum Screen {
    /// Main game view.
    Playing,
    /// Title / main menu.
    Menu,
    /// Between-level stats.
    Intermission,
    /// Game over.
    GameOver,
}

pub enum MenuScreenOption {
	MainMenu,
	SoundMenu,
	ControlMenu
}

pub enum MainMenuOption {
    NewGame,
    Sound,
    Control,
    LoadGame,
    SaveGame,
    ChangeView,
    #[cfg(all(not(feature = "spear"), not(feature = "goodtimes")))]
    ReadThis,
    ViewScores,
    BackToDemo,
    Quit,
}

const MENU_X: i16 = 76;
const MENU_Y: i16 = 55;
const MENU_W: i16 = 178;
#[cfg(not(feature = "spear"))]
const MENU_H: i16 = 13*10+6;
#[cfg(feature = "spear")]
const MENU_H: i16 = 13*9+6;

const SM_X: i16 = 48;
const SM_W: i16 = 250;

const SM_Y1: i16 = 20;
const SM_H1: i16 = 4*13-7;
const SM_Y2: i16 = SM_Y1+5*13;
const SM_H2: i16 = 4*13-7;
const SM_Y3: i16 = SM_Y2+5*13;
const SM_H3: i16 = 3*13-7;

const CTL_X: i16 = 24;
const CTL_Y: i16 = 70;
const CTL_W: i16 = 284;
const CTL_H: i16 = 13*7-7;

const LSM_X: i16 = 85;
const LSM_Y: i16 = 55;
const LSM_W: i16 = 175;
const LSM_H: i16 = 10*13+10;

const NM_X: i16 = 50;
const NM_Y: i16 = 100;
const NM_W: i16 = 225;
const NM_H: i16 = 13*4+15;

const NE_X: i16 = 10;
const NE_Y: i16 = 23;
const NE_W: i16 = 320-NE_X*2;
const NE_H: i16 = 200-NE_Y*2;

const CST_X: i16 = 20;
const CST_Y: i16 = 48;
const CST_START: i16 = 60;
const CST_SPC: i16 = 60;

#[cfg(any(feature = "spear", feature = "goodtimes"))]
const STARTITEM:MainMenuOption = MainMenuOption::NewGame;

#[cfg(all(not(feature = "spear"), not(feature = "goodtimes")))]
const STARTITEM:MainMenuOption = MainMenuOption::ReadThis;

pub struct MenuItemInfo {
    x: i16,
    y: i16,
    amount: u16,
    current_position: i16,
    indent: u16,
}

impl MenuItemInfo {
    fn new(x: i16, y: i16, amount: u16, current_position: i16, indent: u16) -> Self {
        Self {
            x, 
            y,
            amount,
            current_position,
            indent,
        }
    }
}

pub struct MenuItem {
    active: bool,
    title: String,
    handler: fn(i32),
	item_info: MenuItemInfo,
}

impl MenuItem {
    fn new(active: bool, title: String, handler: fn(i32), item_info: MenuItemInfo) -> Self {
        Self {
            active,
            title,
            handler,
			item_info,
        }
    }
}

pub struct MenuScreen {
    items: Vec<MenuItem>,
	current: u16,
}
impl MenuScreen {
    fn new_game(i: i32) {

    }

	fn sound_pc_speaker(i: i32) {

	}

	fn main() -> Self {
		let items = vec![
            MenuItem::new(
				true, String::from("New Game"), Self::new_game,
				MenuItemInfo::new(MENU_X, MENU_Y, 10, 0, 24),
			),
        ];

        Self {
            items,
			current: 0,
        }
	}

	fn sound() -> Self {
		let items = vec![
			MenuItem::new(
				true, String::from("PC Speaker"), Self::sound_pc_speaker,
				MenuItemInfo::new(SM_X, SM_Y1, 12, 0, 52),
			)
		];

		Self {
			items,
			current: 0,
		}
	}
}

pub struct Menu {
	main_menu: MenuScreen,
	sound_menu: MenuScreen,

	current: MenuScreenOption,
}

impl Menu {
	fn new() -> Self {
		let main_menu = MenuScreen::main();
		let sound_menu = MenuScreen::sound();

		Self {
			main_menu,
			sound_menu,
			current: MenuScreenOption::MainMenu,
		}
	}
}

const	PORTTILESWIDE: i16 =		20;      // all drawing takes place inside a
const	PORTTILESHIGH: i16 =		13;		// non displayed port of this size

const UPDATEWIDE: i16 =			PORTTILESWIDE;
const UPDATEHIGH: i16 =			PORTTILESHIGH;

const PIXTOBLOCK: i16 =	4;

// fn VW_MarkUpdateBlock(x1: i16, y1: i16, x2: i16, y2: i16) -> i16 {
// 	let x: i16;
// 	let y: i16;
// 	let mut xt1: i16;
// 	let mut yt1: i16;
// 	let mut xt2: i16;
// 	let mut yt2: i16;
// 	let nextline: i16;
// 	let mark: &u8;

// 	xt1 = x1 >> PIXTOBLOCK;
// 	yt1 = y1 >> PIXTOBLOCK;

// 	xt2 = x2 >> PIXTOBLOCK;
// 	yt2 = y2 >> PIXTOBLOCK;

// 	if xt1 < 0 { xt1 = 0; }
// 	else if xt1 >= UPDATEWIDE { return 0; }
	
// 	if yt1 < 0 { yt1 = 0; }
// 	else if yt1 > UPDATEHIGH { return 0; }

// 	if xt2 < 0 { return 0; }
// 	else if xt2 >= UPDATEWIDE { xt2 = UPDATEWIDE-1; }

// 	if yt2 < 0 { return 0; }
// 	else if yt2 >= UPDATEHIGH { yt2 = UPDATEHIGH-1; }


// 	0 // fix this

// }

// fn VWB_DrawPic(x: i16, y: i16, chunknum: i16, graphics: &GraphicsCache, fb: &mut [u8]) {
//     let picnum = chunknum - STARTPICS;
//     let width: u16;
//     let height: u16;

//     x &= !7; // round down to the nearest multiple of 8

//     let chunk = graphics.chunk(picnum).unwrap();
//     width = chunk.width;
// 	height = chunk.height;


// }

// fn handle_menu(item_i: &MenuItemInfo, items: [MenuItem], handler: fn(i16)) -> u16 {
//     let key: u8;
//     static mut redrawitem: i16 = 1;
//     static mut lastitem: i16 = -1;
//     let i: i16;
//     let x: i16;
//     let basey: i16;
//     let exit: i16;
//     let which: MainMenuOption;
//     let shape: i16;
//     let timer: i16;
//     // let ci: ControlInfo;

//     which = item_i.current_position;
//     x = item_i.x & -8;
//     basey = item_i.y - 2;
//     y = basey + which*13;

//     // VWB_DrawPic(x,y,C_CURSOR1PIC);
//     // SetTextColor(items+which,1);
// 	// if (redrawitem)
// 	// {
// 	// 	PrintX=item_i->x+item_i->indent;
// 	// 	PrintY=item_i->y+which*13;
// 	// 	US_Print((items+which)->string);
// 	// }
// 	// //
// 	// // CALL CUSTOM ROUTINE IF IT IS NEEDED
// 	// //
// 	// if (routine)
// 	// 	routine(which);
// 	// VW_UpdateScreen();

// 	// shape=C_CURSOR1PIC;
// 	// timer=8;
// 	// exit=0;
// 	// TimeCount=0;
// 	// IN_ClearKeysDown();


// 	// do
// 	// {
// 	// 	//
// 	// 	// CHANGE GUN SHAPE
// 	// 	//
// 	// 	if (TimeCount>timer)
// 	// 	{
// 	// 		TimeCount=0;
// 	// 		if (shape==C_CURSOR1PIC)
// 	// 		{
// 	// 			shape=C_CURSOR2PIC;
// 	// 			timer=8;
// 	// 		}
// 	// 		else
// 	// 		{
// 	// 			shape=C_CURSOR1PIC;
// 	// 			timer=70;
// 	// 		}
// 	// 		VWB_DrawPic(x,y,shape);
// 	// 		if (routine)
// 	// 			routine(which);
// 	// 		VW_UpdateScreen();
// 	// 	}

// 	// 	CheckPause();

// 	// 	//
// 	// 	// SEE IF ANY KEYS ARE PRESSED FOR INITIAL CHAR FINDING
// 	// 	//
// 	// 	key=LastASCII;
// 	// 	if (key)
// 	// 	{
// 	// 		int ok=0;

// 	// 		//
// 	// 		// CHECK FOR SCREEN CAPTURE
// 	// 		//
// 	// 		#ifndef SPEAR
// 	// 		if (Keyboard[sc_Tab] && Keyboard[sc_P] && MS_CheckParm("goobers"))
// 	// 		#else
// 	// 		if (Keyboard[sc_Tab] && Keyboard[sc_P] && MS_CheckParm("debugmode"))
// 	// 		#endif
// 	// 			PicturePause();


// 	// 		if (key>='a')
// 	// 			key-='a'-'A';

// 	// 		for (i=which+1;i<item_i->amount;i++)
// 	// 			if ((items+i)->active && (items+i)->string[0]==key)
// 	// 			{
// 	// 				EraseGun(item_i,items,x,y,which);
// 	// 				which=i;
// 	// 				DrawGun(item_i,items,x,&y,which,basey,routine);
// 	// 				ok=1;
// 	// 				IN_ClearKeysDown();
// 	// 				break;
// 	// 			}

// 	// 		//
// 	// 		// DIDN'T FIND A MATCH FIRST TIME THRU. CHECK AGAIN.
// 	// 		//
// 	// 		if (!ok)
// 	// 		{
// 	// 			for (i=0;i<which;i++)
// 	// 				if ((items+i)->active && (items+i)->string[0]==key)
// 	// 				{
// 	// 					EraseGun(item_i,items,x,y,which);
// 	// 					which=i;
// 	// 					DrawGun(item_i,items,x,&y,which,basey,routine);
// 	// 					IN_ClearKeysDown();
// 	// 					break;
// 	// 				}
// 	// 		}
// 	// 	}

// 	// 	//
// 	// 	// GET INPUT
// 	// 	//
// 	// 	ReadAnyControl(&ci);
// 	// 	switch(ci.dir)
// 	// 	{
// 	// 		////////////////////////////////////////////////
// 	// 		//
// 	// 		// MOVE UP
// 	// 		//
// 	// 		case dir_North:

// 	// 		EraseGun(item_i,items,x,y,which);

// 	// 		//
// 	// 		// ANIMATE HALF-STEP
// 	// 		//
// 	// 		if (which && (items+which-1)->active)
// 	// 		{
// 	// 			y-=6;
// 	// 			DrawHalfStep(x,y);
// 	// 		}

// 	// 		//
// 	// 		// MOVE TO NEXT AVAILABLE SPOT
// 	// 		//
// 	// 		do
// 	// 		{
// 	// 			if (!which)
// 	// 				which=item_i->amount-1;
// 	// 			else
// 	// 				which--;
// 	// 		} while(!(items+which)->active);

// 	// 		DrawGun(item_i,items,x,&y,which,basey,routine);
// 	// 		//
// 	// 		// WAIT FOR BUTTON-UP OR DELAY NEXT MOVE
// 	// 		//
// 	// 		TicDelay(20);
// 	// 		break;

// 	// 		////////////////////////////////////////////////
// 	// 		//
// 	// 		// MOVE DOWN
// 	// 		//
// 	// 		case dir_South:

// 	// 		EraseGun(item_i,items,x,y,which);
// 	// 		//
// 	// 		// ANIMATE HALF-STEP
// 	// 		//
// 	// 		if (which!=item_i->amount-1 && (items+which+1)->active)
// 	// 		{
// 	// 			y+=6;
// 	// 			DrawHalfStep(x,y);
// 	// 		}

// 	// 		do
// 	// 		{
// 	// 			if (which==item_i->amount-1)
// 	// 				which=0;
// 	// 			else
// 	// 				which++;
// 	// 		} while(!(items+which)->active);

// 	// 		DrawGun(item_i,items,x,&y,which,basey,routine);

// 	// 		//
// 	// 		// WAIT FOR BUTTON-UP OR DELAY NEXT MOVE
// 	// 		//
// 	// 		TicDelay(20);
// 	// 		break;
// 	// 	}

// 	// 	if (ci.button0 ||
// 	// 		Keyboard[sc_Space] ||
// 	// 		Keyboard[sc_Enter])
// 	// 			exit=1;

// 	// 	if (ci.button1 ||
// 	// 		Keyboard[sc_Escape])
// 	// 			exit=2;

// 	// } while(!exit);


// 	// IN_ClearKeysDown();

// 	// //
// 	// // ERASE EVERYTHING
// 	// //
// 	// if (lastitem!=which)
// 	// {
// 	// 	VWB_Bar(x-1,y,25,16,BKGDCOLOR);
// 	// 	PrintX=item_i->x+item_i->indent;
// 	// 	PrintY=item_i->y+which*13;
// 	// 	US_Print((items+which)->string);
// 	// 	redrawitem=1;
// 	// }
// 	// else
// 	// 	redrawitem=0;

// 	// if (routine)
// 	// 	routine(which);
// 	// VW_UpdateScreen();

// 	// item_i->curpos=which;

// 	// lastitem=which;
// 	// switch(exit)
// 	// {
// 	// 	case 1:
// 	// 		//
// 	// 		// CALL THE ROUTINE
// 	// 		//
// 	// 		if ((items+which)->routine!=NULL)
// 	// 		{
// 	// 			ShootSnd();
// 	// 			MenuFadeOut();
// 	// 			(items+which)->routine(0);
// 	// 		}
// 	// 		return which;

// 	// 	case 2:
// 	// 		SD_PlaySound(ESCPRESSEDSND);
// 	// 		return -1;
// 	// }

// 	// return 0; // JUST TO SHUT UP THE ERROR MESSAGES!
// }

pub struct GameState {
    pub screen: Screen,
    pub player: Player,
    pub map: Option<GameMap>,
    pub actors: ActorList,
    pub doors: DoorList,
    pub renderer: Renderer,
    pub graphics: Option<GraphicsCache>,
    /// Current episode (0-based) and map number (0-based within episode).
    pub episode: usize,
    pub map_num: usize,
    pub tick: u64,
    menu: Menu,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            screen: Screen::Menu,
            player: Player::new(),
            map: None,
            actors: ActorList::new(),
            doors: DoorList::new(),
            renderer: Renderer::new(),
            graphics: None,
            episode: 0,
            map_num: 0,
            tick: 0,
            menu: Menu::new(),
        }
    }

    /// Load all asset data from `base` (e.g. `assets/`).
    /// Call once at startup before the first frame.
    pub fn load_assets(&mut self, base: &Path) {
        match GraphicsCache::load(base) {
            Ok(gc) => self.graphics = Some(gc),
            Err(e) => log::error!("failed to load graphics: {e}"),
        }
    }

    /// Called once per frame — update all game objects.
    pub fn tick(&mut self, input: &InputHandler) {
        self.tick += 1;

        match self.screen {
            Screen::Playing => {
                if let Some(map) = &self.map {
                    self.player.update(input, map);
                }
                self.doors.update();
                self.actors.update_all(&self.player);
            }
            Screen::Menu => {
                // TODO: menu logic
                if input.start_pressed() {
                    self.screen = Screen::Playing;
                }
            }
            Screen::Intermission | Screen::GameOver => {}
        }
    }

    /// Draw the current screen into the RGBA8888 framebuffer.
    pub fn draw(&mut self, fb: &mut [u8], width: usize, height: usize) {
        match self.screen {
            Screen::Playing => self.draw_game(fb, width, height),
            Screen::Menu => self.draw_menu(fb, width, height),
            Screen::Intermission => {}
            Screen::GameOver => {}
        }
    }

    fn draw_game(&mut self, fb: &mut [u8], width: usize, _height: usize) {
        let view = View {
            x: self.player.x,
            y: self.player.y,
            angle: self.player.angle,
        };

        let textures: &[Vec<u8>] = self
            .graphics
            .as_ref()
            .map(|g| g.wall_textures.as_slice())
            .unwrap_or(&[]);

        // Build a flat door-positions slice (indexed by door number).
        let door_pos: Vec<u8> = self.doors.doors.iter().map(|d| d.position).collect();

        if let Some(map) = &self.map {
            self.renderer.draw_frame(fb, width, &view, &map.level, textures, &door_pos);
        } else {
            fb.fill(0);
        }

        // TODO: draw sprites, HUD
    }

    fn draw_menu(&self, fb: &mut [u8], _width: usize, _height: usize) {
        for chunk in fb.chunks_exact_mut(4) {
            chunk.copy_from_slice(&[0x1C, 0x1C, 0x1C, 0xFF]);
        }

		match self.menu.current {
			MenuScreenOption::MainMenu => {
				let screen = &self.menu.main_menu;

				for item in screen.items.iter() {
					println!("menu item {}", item.title);
				}
				// render main menu
				println!("render main menu")
			},
			_ => {
				// todo: fill this in later
			}
		}

        // TODO: blit actual menu graphics
    }

	fn draw_menu_gun(&self, fb: &mut [u8]) {
		
	}

    fn draw_text_string(&self, fb: &mut [u8], text: &str) {
        for c in text.chars().into_iter() {
            // measure char

            // draw char to framebuffer
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
