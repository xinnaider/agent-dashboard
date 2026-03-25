use std::collections::BTreeMap;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
};

use crate::app::App;
use crate::session::{Session, SessionStatus};

// Layout constants
const ROOMS_PER_PAGE: usize = 4;
const SPRITE_W: usize = 10;
const SPRITE_H: usize = 10;
const SPRITE_RENDER_H: u16 = (SPRITE_H as u16).div_ceil(2);
const CHAR_WIDTH: u16 = (SPRITE_W as u16) + 4;
const CHAR_LABEL_LINES: u16 = 4;
const CHAR_HEIGHT: u16 = SPRITE_RENDER_H + CHAR_LABEL_LINES;

type Sprite = [[u8; SPRITE_W]; SPRITE_H];
type Palette = &'static [(u8, u8, u8)];

const PAL_EGG: &[(u8, u8, u8)] = &[
    (0, 0, 0), (255, 250, 230), (220, 200, 170), (180, 220, 180),
];
const SPRITE_EGG: [Sprite; 1] = [[
    [0,0,0,0,1,1,1,0,0,0],[0,0,0,1,1,1,1,1,0,0],[0,0,1,1,1,3,1,1,1,0],
    [0,0,1,1,1,1,1,1,1,0],[0,0,1,3,1,1,1,3,1,0],[0,0,1,1,1,1,1,1,1,0],
    [0,0,1,1,1,1,1,1,1,0],[0,0,0,1,2,1,2,1,0,0],[0,0,0,0,1,1,1,0,0,0],
    [0,0,0,0,0,0,0,0,0,0],
]];

const PAL_WORKING: &[(u8, u8, u8)] = &[
    (0,0,0),(120,220,120),(80,180,80),(40,40,40),(255,255,255),
    (255,150,150),(200,100,80),(100,200,100),(255,220,60),
];
const SPRITE_WORKING: [Sprite; 3] = [
    [[0,0,0,8,1,1,1,8,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,4,1,1,3,4,1,0],[0,1,1,1,1,1,1,1,1,0],[0,5,1,1,6,6,1,1,5,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,7,0,0,7,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,1,1,1,1,0,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,1,3,1,1,3,1,1,0],[0,1,1,1,1,1,1,1,1,0],[0,5,1,6,1,1,6,1,5,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,7,0,0,0,0,7,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,8,1,1,1,1,8,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,4,3,1,1,4,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,5,1,1,6,6,1,1,5,0],
     [8,1,1,1,1,1,1,1,1,8],[0,0,1,1,1,1,1,1,0,0],[0,0,0,7,0,0,7,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

const PAL_IDLE: &[(u8, u8, u8)] = &[
    (0,0,0),(140,160,200),(110,130,170),(60,60,80),(180,190,220),(120,140,180),(200,200,255),
];
const SPRITE_IDLE: [Sprite; 1] = [[
    [0,0,0,1,1,1,1,0,0,0],[0,0,1,1,1,1,1,1,0,6],[0,1,1,1,1,1,1,1,1,0],
    [0,1,3,3,1,1,3,3,1,6],[0,1,1,1,1,1,1,1,1,0],[0,1,1,1,1,1,1,1,1,0],
    [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,5,0,0,5,0,0,0],
    [0,0,0,0,0,0,0,0,0,0],
]];

const PAL_INPUT: &[(u8, u8, u8)] = &[
    (0,0,0),(255,180,60),(220,150,40),(40,40,40),(255,255,255),
    (255,60,60),(200,140,40),(255,100,100),
];
const SPRITE_INPUT: [Sprite; 3] = [
    [[0,0,0,1,1,1,1,0,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,5,1,1,1,1,5,1,0],
     [0,1,1,4,3,3,4,1,1,0],[0,7,1,1,1,1,1,1,7,0],[0,1,1,5,5,5,5,1,1,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,6,0,0,6,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,1,1,1,1,0,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,1,5,1,1,5,1,1,0],
     [0,1,1,4,3,3,4,1,1,0],[0,7,1,1,1,1,1,1,7,0],[0,1,1,1,5,5,1,1,1,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,6,0,0,0,0,6,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,1,1,1,1,0,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,5,1,1,1,1,5,1,0],
     [0,1,1,3,4,4,3,1,1,0],[0,1,7,1,1,1,1,7,1,0],[0,1,5,1,5,5,1,5,1,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,6,0,0,6,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

// ── Pet 1: Cat ───────────────────────────────────────────────────────────────

const PAL_CAT_IDLE: &[(u8, u8, u8)] = &[
    (0,0,0),(230,180,120),(180,120,60),(40,30,20),(255,200,200),(150,90,40),
];
const SPRITE_CAT_IDLE: [Sprite; 1] = [[
    [0,0,2,0,0,0,0,2,0,0],[0,0,1,0,0,0,0,1,0,0],[0,1,1,1,1,1,1,1,1,0],
    [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,1,4,1,2,2,1,4,1,0],
    [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,5,0,0,5,0,0,0],
    [0,0,0,0,0,0,0,0,0,0],
]];

const PAL_CAT_WORKING: &[(u8, u8, u8)] = &[
    (0,0,0),(230,180,120),(180,120,60),(40,30,20),(255,200,200),(150,90,40),(255,240,80),
];
const SPRITE_CAT_WORKING: [Sprite; 3] = [
    [[0,0,2,0,0,0,0,2,0,0],[0,0,1,0,0,0,0,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,6,1,4,2,2,4,1,6,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,5,0,0,0,0,5,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,6,2,0,0,0,0,2,6,0],[0,0,1,0,0,0,0,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,1,1,4,2,2,4,1,1,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,5,0,0,0,0,5,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,2,0,6,6,0,2,0,0],[0,0,1,0,0,0,0,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,6,1,4,2,2,4,1,6,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,5,0,0,0,0,5,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

const PAL_CAT_INPUT: &[(u8, u8, u8)] = &[
    (0,0,0),(230,180,120),(180,120,60),(40,30,20),(255,200,200),(150,90,40),(255,230,50),(255,100,100),
];
const SPRITE_CAT_INPUT: [Sprite; 3] = [
    [[0,0,2,0,7,7,0,2,0,0],[0,0,1,0,0,0,0,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,3,1,1,3,3,1,0],[0,7,1,1,1,1,1,1,7,0],[0,1,1,4,2,2,4,1,1,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,5,0,0,5,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,2,0,6,6,0,2,0,0],[0,0,1,0,0,0,0,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,3,1,1,3,3,1,0],[0,1,7,1,1,1,1,7,1,0],[0,1,1,4,2,2,4,1,1,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,6,0,0,0,0,6,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,6,2,0,0,0,0,2,6,0],[0,0,1,0,0,0,0,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,3,1,1,3,3,1,0],[0,7,1,1,1,1,1,1,7,0],[0,1,1,4,2,2,4,1,1,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,5,0,0,5,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

// ── Pet 2: Robot ─────────────────────────────────────────────────────────────

const PAL_ROBOT_IDLE: &[(u8, u8, u8)] = &[
    (0,0,0),(160,170,180),(120,130,140),(80,200,255),(200,210,220),(100,110,120),
];
const SPRITE_ROBOT_IDLE: [Sprite; 1] = [[
    [0,0,0,0,4,0,0,0,0,0],[0,0,0,0,2,0,0,0,0,0],[0,0,1,1,1,1,1,1,0,0],
    [0,0,1,3,1,1,3,1,0,0],[0,0,1,1,1,1,1,1,0,0],[0,0,1,2,2,2,2,1,0,0],
    [0,0,1,1,1,1,1,1,0,0],[0,0,0,1,1,1,1,0,0,0],[0,0,0,5,0,0,5,0,0,0],
    [0,0,0,0,0,0,0,0,0,0],
]];

const PAL_ROBOT_WORKING: &[(u8, u8, u8)] = &[
    (0,0,0),(160,170,180),(120,130,140),(80,200,255),(200,210,220),(100,110,120),(255,200,50),(255,255,255),
];
const SPRITE_ROBOT_WORKING: [Sprite; 3] = [
    [[0,0,0,6,4,0,0,0,0,0],[0,0,0,0,2,0,0,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,0,1,3,1,1,3,1,0,0],[0,0,1,1,1,1,1,1,0,0],[0,7,1,2,2,2,2,1,7,0],
     [0,0,1,1,1,1,1,1,0,0],[0,0,0,1,1,1,1,0,0,0],[0,0,5,0,0,0,0,5,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,0,4,6,0,0,0,0],[0,0,0,0,2,0,0,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,0,1,6,1,1,6,1,0,0],[0,0,1,1,1,1,1,1,0,0],[0,0,1,2,6,6,2,1,0,0],
     [0,0,1,1,1,1,1,1,0,0],[0,0,0,1,1,1,1,0,0,0],[0,0,5,0,0,0,0,5,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,7,0,0,4,0,0,7,0,0],[0,0,0,0,2,0,0,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,0,1,3,1,1,3,1,0,0],[0,0,1,1,1,1,1,1,0,0],[0,7,1,2,2,2,2,1,7,0],
     [0,0,1,1,1,1,1,1,0,0],[0,0,0,1,1,1,1,0,0,0],[0,0,5,0,0,0,0,5,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

const PAL_ROBOT_INPUT: &[(u8, u8, u8)] = &[
    (0,0,0),(160,170,180),(120,130,140),(80,200,255),(200,210,220),(100,110,120),(255,100,100),(255,200,200),
];
const SPRITE_ROBOT_INPUT: [Sprite; 3] = [
    [[0,0,0,6,4,6,0,0,0,0],[0,0,0,0,2,0,0,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,0,1,6,1,1,6,1,0,0],[0,6,1,1,1,1,1,1,6,0],[0,0,1,2,2,2,2,1,0,0],
     [0,0,1,1,1,1,1,1,0,0],[0,0,0,1,1,1,1,0,0,0],[0,0,0,5,0,0,5,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,0,6,0,0,0,0,0],[0,0,0,0,2,0,0,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,0,1,3,1,1,3,1,0,0],[0,0,1,1,1,1,1,1,0,0],[0,0,1,6,2,2,6,1,0,0],
     [0,0,1,1,1,1,1,1,0,0],[0,0,0,1,1,1,1,0,0,0],[0,0,6,0,0,0,0,6,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,6,4,6,0,0,0,0],[0,0,0,0,2,0,0,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,0,1,6,1,1,6,1,0,0],[0,6,1,1,1,1,1,1,6,0],[0,0,1,2,2,2,2,1,0,0],
     [0,0,1,1,1,1,1,1,0,0],[0,0,0,1,1,1,1,0,0,0],[0,0,0,5,0,0,5,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

// ── Pet 3: Ghost ─────────────────────────────────────────────────────────────

const PAL_GHOST_IDLE: &[(u8, u8, u8)] = &[
    (0,0,0),(220,215,255),(180,170,220),(60,40,80),(255,255,255),(140,120,180),
];
const SPRITE_GHOST_IDLE: [Sprite; 1] = [[
    [0,0,0,1,1,1,1,0,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,1,1,1,1,1,1,1,0],
    [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,1,1,1,1,1,1,1,1,0],
    [0,1,2,1,2,2,1,2,1,0],[0,0,1,0,1,1,0,1,0,0],[0,0,0,0,0,0,0,0,0,0],
    [0,0,0,0,0,0,0,0,0,0],
]];

const PAL_GHOST_WORKING: &[(u8, u8, u8)] = &[
    (0,0,0),(220,215,255),(180,170,220),(60,40,80),(255,255,255),(140,120,180),(255,220,100),(200,190,255),
];
const SPRITE_GHOST_WORKING: [Sprite; 3] = [
    [[0,0,0,1,1,1,1,0,0,0],[0,0,1,1,1,1,1,1,0,0],[0,6,1,1,1,1,1,1,6,0],
     [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,2,1,2,2,1,2,1,0],[0,0,1,6,1,1,6,1,0,0],[0,0,0,0,0,0,0,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,6,0,1,1,1,1,0,6,0],[0,0,1,1,1,1,1,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,7,1,1,1,1,1,1,7,0],
     [0,1,2,1,2,2,1,2,1,0],[0,0,1,0,1,1,0,1,0,0],[0,0,0,0,0,0,0,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,6,1,1,1,1,6,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,1,1,1,1,1,1,1,1,0],
     [0,6,2,1,2,2,1,2,6,0],[0,0,1,0,1,1,0,1,0,0],[0,0,0,0,0,0,0,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

const PAL_GHOST_INPUT: &[(u8, u8, u8)] = &[
    (0,0,0),(220,215,255),(180,170,220),(60,40,80),(255,255,255),(140,120,180),(255,100,150),(255,200,230),
];
const SPRITE_GHOST_INPUT: [Sprite; 3] = [
    [[0,0,0,1,1,1,1,0,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,3,1,1,3,3,1,0],[0,6,1,1,1,1,1,1,6,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,2,1,2,2,1,2,1,0],[0,6,1,0,1,1,0,1,6,0],[0,0,0,0,0,0,0,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,1,1,1,1,0,0,0],[0,0,1,1,1,1,1,1,0,0],[0,7,1,1,1,1,1,1,7,0],
     [0,1,3,3,1,1,3,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,6,1,1,1,1,1,1,6,0],
     [0,1,2,1,2,2,1,2,1,0],[0,0,1,0,1,1,0,1,0,0],[0,0,0,0,0,0,0,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,6,1,1,1,1,6,0,0],[0,0,1,1,1,1,1,1,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,3,1,1,3,3,1,0],[0,6,1,1,1,1,1,1,6,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,2,1,2,2,1,2,1,0],[0,6,1,0,1,1,0,1,6,0],[0,0,0,0,0,0,0,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

// ── Pet 4: Slime ─────────────────────────────────────────────────────────────

const PAL_SLIME_IDLE: &[(u8, u8, u8)] = &[
    (0,0,0),(160,230,80),(120,190,40),(40,80,20),(220,255,140),(80,160,30),
];
const SPRITE_SLIME_IDLE: [Sprite; 1] = [[
    [0,0,0,0,0,0,0,0,0,0],[0,0,0,4,4,4,4,0,0,0],[0,0,1,1,1,1,1,1,0,0],
    [0,1,1,3,1,1,3,1,1,0],[0,1,1,1,1,1,1,1,1,0],[0,1,1,1,2,2,1,1,1,0],
    [1,1,1,1,1,1,1,1,1,1],[0,1,1,2,1,1,2,1,1,0],[0,0,2,0,0,0,0,2,0,0],
    [0,0,0,0,0,0,0,0,0,0],
]];

const PAL_SLIME_WORKING: &[(u8, u8, u8)] = &[
    (0,0,0),(160,230,80),(120,190,40),(40,80,20),(220,255,140),(80,160,30),(255,255,100),(200,255,80),
];
const SPRITE_SLIME_WORKING: [Sprite; 3] = [
    [[0,0,0,0,1,1,0,0,0,0],[0,0,0,4,1,1,4,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,1,1,3,1,1,3,1,1,0],[0,1,1,1,1,1,1,1,1,0],[0,6,1,1,2,2,1,1,6,0],
     [1,1,1,1,1,1,1,1,1,1],[0,1,2,1,1,1,1,2,1,0],[0,0,0,0,0,0,0,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,0,0,0,0,0,0,0],[0,0,0,6,4,4,6,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,1,1,3,1,1,3,1,1,0],[0,1,7,1,1,1,1,7,1,0],[0,1,1,1,2,2,1,1,1,0],
     [1,1,1,1,1,1,1,1,1,1],[0,1,1,2,0,0,2,1,1,0],[0,0,2,0,0,0,0,2,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,6,1,1,6,0,0,0],[0,0,0,4,1,1,4,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,1,1,3,1,1,3,1,1,0],[0,6,1,1,1,1,1,1,6,0],[0,1,1,1,2,2,1,1,1,0],
     [1,1,1,1,1,1,1,1,1,1],[0,1,2,1,1,1,1,2,1,0],[0,0,0,0,0,0,0,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

const PAL_SLIME_INPUT: &[(u8, u8, u8)] = &[
    (0,0,0),(160,230,80),(120,190,40),(40,80,20),(220,255,140),(80,160,30),(255,200,50),(255,150,50),
];
const SPRITE_SLIME_INPUT: [Sprite; 3] = [
    [[0,0,0,0,6,6,0,0,0,0],[0,0,0,4,1,1,4,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,1,7,3,3,3,3,7,1,0],[0,1,1,1,1,1,1,1,1,0],[0,6,1,1,2,2,1,1,6,0],
     [1,1,1,1,1,1,1,1,1,1],[0,1,2,1,1,1,1,2,1,0],[0,0,2,0,0,0,0,2,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,6,0,0,6,0,0,0],[0,0,0,4,4,4,4,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,1,1,3,3,3,3,1,1,0],[0,7,1,1,1,1,1,1,7,0],[0,1,1,1,2,2,1,1,1,0],
     [1,1,1,1,1,1,1,1,1,1],[0,1,1,2,0,0,2,1,1,0],[0,0,6,0,0,0,0,6,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,0,0,6,6,0,0,0,0],[0,0,0,4,1,1,4,0,0,0],[0,0,1,1,1,1,1,1,0,0],
     [0,1,7,3,3,3,3,7,1,0],[0,1,1,1,1,1,1,1,1,0],[0,6,1,1,2,2,1,1,6,0],
     [1,1,1,1,1,1,1,1,1,1],[0,1,2,1,1,1,1,2,1,0],[0,0,2,0,0,0,0,2,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

// ── Pet 5: Dragon ────────────────────────────────────────────────────────────

const PAL_DRAGON_IDLE: &[(u8, u8, u8)] = &[
    (0,0,0),(220,80,60),(180,40,20),(40,20,10),(255,200,100),(255,120,40),(120,200,60),
];
const SPRITE_DRAGON_IDLE: [Sprite; 1] = [[
    [0,0,6,0,0,0,0,6,0,0],[0,0,2,0,0,0,0,2,0,0],[0,1,1,1,1,1,1,1,1,0],
    [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,5,1,4,1,1,4,1,5,0],
    [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,2,0,0,2,0,0,0],
    [0,0,0,0,0,0,0,0,0,0],
]];

const PAL_DRAGON_WORKING: &[(u8, u8, u8)] = &[
    (0,0,0),(220,80,60),(180,40,20),(40,20,10),(255,200,100),(255,120,40),(120,200,60),(255,240,60),(255,160,20),
];
const SPRITE_DRAGON_WORKING: [Sprite; 3] = [
    [[0,0,6,0,7,7,0,6,0,0],[0,0,2,0,0,0,0,2,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,4,1,1,4,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,5,1,4,1,1,4,1,5,0],
     [0,8,1,1,1,1,1,1,8,0],[0,0,1,1,1,1,1,1,0,0],[0,0,7,2,0,0,2,7,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,7,6,0,0,0,0,6,7,0],[0,0,2,0,0,0,0,2,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,1,1,1,1,3,1,0],[0,1,1,1,1,1,1,1,1,0],[0,5,8,4,1,1,4,8,5,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,2,0,0,0,0,2,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,6,7,0,0,7,6,0,0],[0,0,2,0,0,0,0,2,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,4,3,1,1,3,4,1,0],[0,1,1,1,1,1,1,1,1,0],[0,5,1,4,8,8,4,1,5,0],
     [0,8,1,1,1,1,1,1,8,0],[0,0,1,1,1,1,1,1,0,0],[0,7,2,0,0,0,0,2,7,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

const PAL_DRAGON_INPUT: &[(u8, u8, u8)] = &[
    (0,0,0),(220,80,60),(180,40,20),(40,20,10),(255,200,100),(255,120,40),(120,200,60),(255,60,60),(255,200,200),
];
const SPRITE_DRAGON_INPUT: [Sprite; 3] = [
    [[0,0,6,0,7,7,0,6,0,0],[0,0,2,0,0,0,0,2,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,3,1,1,3,3,1,0],[0,7,1,1,1,1,1,1,7,0],[0,5,1,4,1,1,4,1,5,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,2,0,0,2,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,7,6,0,0,0,0,6,7,0],[0,0,2,0,0,0,0,2,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,3,1,1,3,3,1,0],[0,1,7,1,1,1,1,7,1,0],[0,5,1,4,1,1,4,1,5,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,7,2,0,0,2,7,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
    [[0,0,6,0,7,7,0,6,0,0],[0,0,2,0,0,0,0,2,0,0],[0,1,1,1,1,1,1,1,1,0],
     [0,1,3,3,1,1,3,3,1,0],[0,7,1,1,1,1,1,1,7,0],[0,5,1,4,1,1,4,1,5,0],
     [0,1,1,1,1,1,1,1,1,0],[0,0,1,1,1,1,1,1,0,0],[0,0,0,2,0,0,2,0,0,0],
     [0,0,0,0,0,0,0,0,0,0]],
];

fn sprite_data(status: &SessionStatus, frame: usize, pet: usize) -> (&'static Sprite, Palette) {
    match status {
        SessionStatus::New => (&SPRITE_EGG[0], PAL_EGG),
        SessionStatus::Working => match pet {
            1 => (&SPRITE_CAT_WORKING[frame % 3], PAL_CAT_WORKING),
            2 => (&SPRITE_ROBOT_WORKING[frame % 3], PAL_ROBOT_WORKING),
            3 => (&SPRITE_GHOST_WORKING[frame % 3], PAL_GHOST_WORKING),
            4 => (&SPRITE_SLIME_WORKING[frame % 3], PAL_SLIME_WORKING),
            5 => (&SPRITE_DRAGON_WORKING[frame % 3], PAL_DRAGON_WORKING),
            _ => (&SPRITE_WORKING[frame % 3], PAL_WORKING),
        },
        SessionStatus::Idle => match pet {
            1 => (&SPRITE_CAT_IDLE[0], PAL_CAT_IDLE),
            2 => (&SPRITE_ROBOT_IDLE[0], PAL_ROBOT_IDLE),
            3 => (&SPRITE_GHOST_IDLE[0], PAL_GHOST_IDLE),
            4 => (&SPRITE_SLIME_IDLE[0], PAL_SLIME_IDLE),
            5 => (&SPRITE_DRAGON_IDLE[0], PAL_DRAGON_IDLE),
            _ => (&SPRITE_IDLE[0], PAL_IDLE),
        },
        SessionStatus::Input => match pet {
            1 => (&SPRITE_CAT_INPUT[frame % 3], PAL_CAT_INPUT),
            2 => (&SPRITE_ROBOT_INPUT[frame % 3], PAL_ROBOT_INPUT),
            3 => (&SPRITE_GHOST_INPUT[frame % 3], PAL_GHOST_INPUT),
            4 => (&SPRITE_SLIME_INPUT[frame % 3], PAL_SLIME_INPUT),
            5 => (&SPRITE_DRAGON_INPUT[frame % 3], PAL_DRAGON_INPUT),
            _ => (&SPRITE_INPUT[frame % 3], PAL_INPUT),
        },
    }
}

fn render_sprite_lines(sprite: &Sprite, palette: Palette) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for y in (0..SPRITE_H).step_by(2) {
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (x, &top) in sprite[y].iter().enumerate().take(SPRITE_W) {
            let bot = if y + 1 < SPRITE_H { sprite[y + 1][x] } else { 0 };
            if top == 0 && bot == 0 {
                spans.push(Span::raw(" "));
            } else if top == 0 {
                let (r, g, b) = palette[bot as usize];
                spans.push(Span::styled("\u{2584}", Style::default().fg(Color::Rgb(r, g, b))));
            } else if bot == 0 {
                let (r, g, b) = palette[top as usize];
                spans.push(Span::styled("\u{2580}", Style::default().fg(Color::Rgb(r, g, b))));
            } else {
                let (tr, tg, tb) = palette[top as usize];
                let (br, bg, bb) = palette[bot as usize];
                spans.push(Span::styled(
                    "\u{2580}",
                    Style::default().fg(Color::Rgb(tr, tg, tb)).bg(Color::Rgb(br, bg, bb)),
                ));
            }
        }
        lines.push(Line::from(spans));
    }
    lines
}

struct Room {
    name: String,
    session_indices: Vec<usize>,
    has_input: bool,
    last_activity: Option<String>,
}

fn group_into_rooms(sessions: &[Session]) -> Vec<Room> {
    let mut map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, s) in sessions.iter().enumerate() {
        let room_name = if s.project_name.is_empty() {
            "unknown".to_string()
        } else {
            s.room_id()
        };
        map.entry(room_name).or_default().push(i);
    }
    let mut rooms: Vec<Room> = map
        .into_iter()
        .map(|(name, indices)| {
            let has_input = indices.iter().any(|&i| sessions[i].status == SessionStatus::Input);
            let last_activity = indices.iter().filter_map(|&i| sessions[i].last_activity.as_ref()).max().cloned();
            Room { name, session_indices: indices, has_input, last_activity }
        })
        .collect();
    rooms.sort_by(|a, b| {
        b.has_input.cmp(&a.has_input).then_with(|| b.last_activity.cmp(&a.last_activity))
    });
    rooms
}

fn animation_frame(status: &SessionStatus, tick: u64) -> usize {
    match status {
        SessionStatus::Working => ((tick / 2) % 3) as usize,
        SessionStatus::Input => (tick % 3) as usize,
        _ => 0,
    }
}

fn session_phase_offset(session_id: &str) -> u64 {
    session_id.bytes().fold(0u64, |a, b| a.wrapping_add(b as u64)) % 7
}

fn pet_type(session_id: &str) -> usize {
    session_id.bytes().fold(0u64, |a, b| a.wrapping_add(b as u64)) as usize % 6
}

fn status_color(status: &SessionStatus) -> Color {
    match status {
        SessionStatus::New => Color::Blue,
        SessionStatus::Working => Color::Green,
        SessionStatus::Idle => Color::DarkGray,
        SessionStatus::Input => Color::Yellow,
    }
}

fn action_or_label(session: &Session) -> String {
    match session.status {
        SessionStatus::Working | SessionStatus::Input => {
            if let Some(ref action) = session.last_action {
                return action.clone();
            }
            session.status.label().to_string()
        }
        _ => session.status.label().to_string(),
    }
}

fn context_bar(ratio: f64) -> (String, Color) {
    let bar_width = 6usize;
    let filled = (ratio * bar_width as f64).round().min(bar_width as f64) as usize;
    let empty = bar_width - filled;
    let pct = (ratio * 100.0) as u32;
    let bar = format!("{}{} {}%", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty), pct);
    let color = if ratio > 0.75 { Color::Red } else if ratio > 0.40 { Color::Yellow } else { Color::Green };
    (bar, color)
}

pub fn resolve_zoom(app: &mut App) {
    let rooms = group_into_rooms(&app.sessions);
    let total_pages = rooms.len().div_ceil(ROOMS_PER_PAGE);
    if total_pages > 0 {
        app.view_page = app.view_page.min(total_pages - 1);
    } else {
        app.view_page = 0;
    }
    if let Some(idx) = app.view_zoom_index.take() {
        let page_start = app.view_page * ROOMS_PER_PAGE;
        if let Some(room) = rooms.get(page_start + idx) {
            app.view_zoomed_room = Some(room.name.clone());
        }
    }
    if let Some(ref zoomed_name) = app.view_zoomed_room {
        if let Some(room) = rooms.iter().find(|r| &r.name == zoomed_name) {
            if !room.session_indices.is_empty() {
                app.view_selected_agent = app.view_selected_agent.min(room.session_indices.len() - 1);
            } else {
                app.view_selected_agent = 0;
            }
        }
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
    render_rooms(frame, app, chunks[0]);
    render_footer(frame, app, chunks[1]);
}

fn render_rooms(frame: &mut Frame, app: &App, area: Rect) {
    let rooms = group_into_rooms(&app.sessions);
    if rooms.is_empty() {
        render_empty(frame, area, app.tick);
        return;
    }
    if let Some(ref zoomed_name) = app.view_zoomed_room {
        if let Some(room) = rooms.iter().find(|r| &r.name == zoomed_name) {
            render_room(frame, app, room, area, None, Some(app.view_selected_agent));
            return;
        }
    }
    let total_pages = rooms.len().div_ceil(ROOMS_PER_PAGE);
    let page = app.view_page.min(total_pages.saturating_sub(1));
    let page_start = page * ROOMS_PER_PAGE;
    let page_rooms: Vec<&Room> = rooms.iter().skip(page_start).take(ROOMS_PER_PAGE).collect();
    let v_chunks = Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);
    let top_h = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(v_chunks[0]);
    let bot_h = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(v_chunks[1]);
    let cells = [top_h[0], top_h[1], bot_h[0], bot_h[1]];
    for (i, cell) in cells.iter().enumerate() {
        if let Some(room) = page_rooms.get(i) {
            render_room(frame, app, room, *cell, Some(i + 1), None);
        } else {
            let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(30, 30, 30)));
            frame.render_widget(block, *cell);
        }
    }
}

fn render_room(frame: &mut Frame, app: &App, room: &Room, area: Rect, slot_num: Option<usize>, selected_agent: Option<usize>) {
    let border_color = if room.has_input {
        if app.tick.is_multiple_of(2) { Color::Yellow } else { Color::White }
    } else {
        Color::DarkGray
    };
    let title = match slot_num {
        Some(n) => format!(" [{}] {} ({}) ", n, room.name, room.session_indices.len()),
        None => format!(" {} ({}) ", room.name, room.session_indices.len()),
    };
    let title_style = if room.has_input { Style::default().fg(border_color) } else { Style::default().fg(Color::White) };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, title_style))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 { return; }
    let chars_per_row = (inner.width / CHAR_WIDTH).max(1) as usize;
    let char_rows: Vec<&[usize]> = room.session_indices.chunks(chars_per_row).collect();
    let needed_height = char_rows.len() as u16 * CHAR_HEIGHT;
    let v_pad = inner.height.saturating_sub(needed_height) / 2;
    let char_area = Rect { x: inner.x, y: inner.y + v_pad, width: inner.width, height: inner.height.saturating_sub(v_pad) };
    let row_constraints: Vec<Constraint> = char_rows.iter().map(|_| Constraint::Length(CHAR_HEIGHT)).collect();
    let v_chunks = Layout::vertical(row_constraints).split(char_area);
    for (row_idx, indices) in char_rows.iter().enumerate() {
        if row_idx >= v_chunks.len() { break; }
        let col_constraints: Vec<Constraint> = indices.iter().map(|_| Constraint::Length(CHAR_WIDTH)).collect();
        let h_chunks = Layout::horizontal(col_constraints).split(v_chunks[row_idx]);
        for (col_idx, &session_idx) in indices.iter().enumerate() {
            if col_idx >= h_chunks.len() { break; }
            let flat_idx = row_idx * chars_per_row + col_idx;
            let is_selected = selected_agent == Some(flat_idx);
            render_character(frame, &app.sessions[session_idx], h_chunks[col_idx], app.tick, is_selected);
        }
    }
}

fn render_character(frame: &mut Frame, session: &Session, area: Rect, tick: u64, is_selected: bool) {
    if area.height < 3 || area.width < 4 { return; }
    let offset = session_phase_offset(&session.session_id);
    let anim_frame = animation_frame(&session.status, tick + offset);
    let pet = pet_type(&session.session_id);
    let (sprite, palette) = sprite_data(&session.status, anim_frame, pet);
    let ratio = session.token_ratio();
    let color = if session.status == SessionStatus::Input {
        if tick.is_multiple_of(2) { Color::Yellow } else { Color::White }
    } else {
        status_color(&session.status)
    };
    if is_selected {
        let bg = Block::default().style(Style::default().bg(Color::Rgb(40, 40, 60)));
        frame.render_widget(bg, area);
    }
    let mut lines: Vec<Line> = Vec::new();
    lines.extend(render_sprite_lines(sprite, palette));
    let name = session.pid.map(|p| p.to_string()).unwrap_or_else(|| "???".to_string());
    let name_style = if is_selected {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    lines.push(Line::from(Span::styled(truncate_str(&name, area.width as usize), name_style)));
    let branch = session.branch.as_deref().unwrap_or("");
    lines.push(Line::from(Span::styled(truncate_str(branch, area.width as usize), Style::default().fg(Color::Green))));
    lines.push(Line::from(Span::styled(truncate_str(&action_or_label(session), area.width as usize), Style::default().fg(color))));
    let (bar_str, bar_color) = context_bar(ratio);
    lines.push(Line::from(Span::styled(truncate_str(&bar_str, area.width as usize), Style::default().fg(bar_color))));
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

fn render_empty(frame: &mut Frame, area: Rect, _tick: u64) {
    let (sprite, palette) = sprite_data(&SessionStatus::Idle, 0, 0);
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.extend(render_sprite_lines(sprite, palette));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("No active sessions", Style::default().fg(Color::DarkGray))));
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let rooms = group_into_rooms(&app.sessions);
    let total_pages = rooms.len().div_ceil(ROOMS_PER_PAGE);
    let page = app.view_page.min(total_pages.saturating_sub(1));
    let mut spans = vec![];
    if app.view_zoomed_room.is_some() {
        spans.push(Span::styled("h/l", Style::default().fg(Color::Cyan)));
        spans.push(Span::raw(" select  "));
        spans.push(Span::styled("Esc", Style::default().fg(Color::Cyan)));
        spans.push(Span::raw(" back  "));
    } else {
        spans.push(Span::styled("1-4", Style::default().fg(Color::Cyan)));
        spans.push(Span::raw(" zoom  "));
        if total_pages > 1 {
            spans.push(Span::styled("j/k", Style::default().fg(Color::Cyan)));
            spans.push(Span::raw(format!(" page ({}/{})  ", page + 1, total_pages)));
        }
    }
    spans.push(Span::styled("v", Style::default().fg(Color::Cyan)));
    spans.push(Span::raw(" table  "));
    spans.push(Span::styled("q", Style::default().fg(Color::Cyan)));
    spans.push(Span::raw(" quit"));
    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}

fn truncate_str(s: &str, max_width: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_width {
        s.to_string()
    } else if max_width > 1 {
        let truncated: String = s.chars().take(max_width - 1).collect();
        format!("{}\u{2026}", truncated)
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(cwd: &str, status: SessionStatus, last_activity: Option<&str>) -> Session {
        Session {
            session_id: String::new(),
            project_name: cwd.to_string(),
            branch: None,
            cwd: cwd.to_string(),
            relative_dir: None,
            model: None,
            effort: None,
            total_input_tokens: 0,
            total_output_tokens: 0,
            status,
            pid: None,
            last_activity: last_activity.map(|s| s.to_string()),
            last_action: None,
            started_at: 0,
            last_file_size: 0,
        }
    }

    #[test]
    fn status_label_shows_action_when_working_with_action() {
        // When Working + last_action present, show action
        let mut s = make_session("/a", SessionStatus::Working, Some("2026-03-24T10:00:00Z"));
        s.last_action = Some("Edit main.rs".to_string());
        assert_eq!(action_or_label(&s), "Edit main.rs");
    }

    #[test]
    fn status_label_shows_working_when_no_action() {
        let s = make_session("/a", SessionStatus::Working, Some("2026-03-24T10:00:00Z"));
        assert_eq!(action_or_label(&s), "Working");
    }

    #[test]
    fn status_label_shows_idle_label_even_with_action() {
        // Idle sessions never show action
        let mut s = make_session("/a", SessionStatus::Idle, Some("2026-03-24T10:00:00Z"));
        s.last_action = Some("Edit main.rs".to_string());
        assert_eq!(action_or_label(&s), "Idle");
    }

    #[test]
    fn status_label_shows_new_label_even_with_action() {
        // New sessions never show action
        let mut s = make_session("/a", SessionStatus::New, None);
        s.last_action = Some("Edit main.rs".to_string());
        assert_eq!(action_or_label(&s), "New");
    }

    #[test]
    fn status_label_shows_action_when_input_with_action() {
        let mut s = make_session("/a", SessionStatus::Input, Some("2026-03-24T10:00:00Z"));
        s.last_action = Some("Read file.rs".to_string());
        assert_eq!(action_or_label(&s), "Read file.rs");
    }

    #[test]
    fn rooms_with_input_sort_first() {
        let sessions = vec![
            make_session("/a", SessionStatus::Idle, Some("2026-03-16T10:00:00Z")),
            make_session("/b", SessionStatus::Input, Some("2026-03-16T09:00:00Z")),
        ];
        let rooms = group_into_rooms(&sessions);
        assert_eq!(rooms[0].name, "/b");
        assert_eq!(rooms[1].name, "/a");
    }

    #[test]
    fn secondary_sort_by_last_activity_descending() {
        let sessions = vec![
            make_session("/old", SessionStatus::Idle, Some("2026-03-16T08:00:00Z")),
            make_session("/recent", SessionStatus::Idle, Some("2026-03-16T12:00:00Z")),
            make_session("/mid", SessionStatus::Idle, Some("2026-03-16T10:00:00Z")),
        ];
        let rooms = group_into_rooms(&sessions);
        assert_eq!(rooms[0].name, "/recent");
        assert_eq!(rooms[1].name, "/mid");
        assert_eq!(rooms[2].name, "/old");
    }

    #[test]
    fn new_sessions_sort_last() {
        let sessions = vec![
            make_session("/egg", SessionStatus::New, None),
            make_session("/active", SessionStatus::Idle, Some("2026-03-16T10:00:00Z")),
        ];
        let rooms = group_into_rooms(&sessions);
        assert_eq!(rooms[0].name, "/active");
        assert_eq!(rooms[1].name, "/egg");
    }

    #[test]
    fn room_activity_uses_max_across_sessions() {
        let sessions = vec![
            make_session("/repo", SessionStatus::Idle, Some("2026-03-16T08:00:00Z")),
            make_session("/repo", SessionStatus::New, None),
            make_session("/repo", SessionStatus::Idle, Some("2026-03-16T12:00:00Z")),
            make_session("/other", SessionStatus::Idle, Some("2026-03-16T10:00:00Z")),
        ];
        let rooms = group_into_rooms(&sessions);
        assert_eq!(rooms[0].name, "/repo");
        assert_eq!(rooms[1].name, "/other");
    }

    #[test]
    fn input_rooms_also_sorted_by_activity() {
        let sessions = vec![
            make_session("/old-input", SessionStatus::Input, Some("2026-03-16T08:00:00Z")),
            make_session("/new-input", SessionStatus::Input, Some("2026-03-16T12:00:00Z")),
        ];
        let rooms = group_into_rooms(&sessions);
        assert_eq!(rooms[0].name, "/new-input");
        assert_eq!(rooms[1].name, "/old-input");
    }

    #[test]
    fn worktrees_share_room_by_project_name() {
        let mut s1 = make_session("/repos/line5", SessionStatus::Idle, Some("2026-03-16T10:00:00Z"));
        s1.project_name = "line5".to_string();
        let mut s2 = make_session("/worktrees/line5-feat", SessionStatus::Working, Some("2026-03-16T11:00:00Z"));
        s2.project_name = "line5".to_string();
        let rooms = group_into_rooms(&[s1, s2]);
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "line5");
        assert_eq!(rooms[0].session_indices.len(), 2);
    }

    #[test]
    fn subproject_gets_separate_room() {
        let mut s1 = make_session("/repos/line5", SessionStatus::Idle, Some("2026-03-16T10:00:00Z"));
        s1.project_name = "line5".to_string();
        let mut s2 = make_session("/repos/line5/tools/solo", SessionStatus::Idle, Some("2026-03-16T11:00:00Z"));
        s2.project_name = "line5".to_string();
        s2.relative_dir = Some("tools/solo".to_string());
        let rooms = group_into_rooms(&[s1, s2]);
        assert_eq!(rooms.len(), 2);
    }

    #[test]
    fn mixed_input_and_activity_sorting() {
        let sessions = vec![
            make_session("/idle-recent", SessionStatus::Idle, Some("2026-03-16T15:00:00Z")),
            make_session("/input-old", SessionStatus::Input, Some("2026-03-16T08:00:00Z")),
            make_session("/egg", SessionStatus::New, None),
            make_session("/idle-old", SessionStatus::Idle, Some("2026-03-16T09:00:00Z")),
        ];
        let rooms = group_into_rooms(&sessions);
        assert_eq!(rooms[0].name, "/input-old");
        assert_eq!(rooms[1].name, "/idle-recent");
        assert_eq!(rooms[2].name, "/idle-old");
        assert_eq!(rooms[3].name, "/egg");
    }

    #[test]
    fn pet_type_is_in_range() {
        // Any session_id maps to a value in [0, 5]
        assert!(pet_type("abc-123") < 6);
        assert!(pet_type("") < 6);
        assert!(pet_type("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa") < 6);
    }

    #[test]
    fn pet_type_is_deterministic() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(pet_type(id), pet_type(id));
    }

    #[test]
    fn pet_type_varies_across_ids() {
        // With 6 types, a sample of 12 IDs should produce at least 2 distinct values
        let ids = ["a","b","c","d","e","f","g","h","i","j","k","l"];
        let types: std::collections::HashSet<usize> = ids.iter().map(|id| pet_type(id)).collect();
        assert!(types.len() >= 2);
    }

    #[test]
    fn sprite_data_returns_egg_for_new_regardless_of_pet() {
        // All pets show egg when status is New — pet param ignored
        for pet in 0..6 {
            let (sprite, _pal) = sprite_data(&SessionStatus::New, 0, pet);
            // Egg sprite has empty bottom row — stable property
            assert_eq!(sprite[9], [0u8; 10]);
        }
    }

    #[test]
    fn sprite_data_different_pets_return_different_palettes_for_idle() {
        let (_, pal0) = sprite_data(&SessionStatus::Idle, 0, 0);
        let (_, pal1) = sprite_data(&SessionStatus::Idle, 0, 1);
        // Frog and Cat palettes have different body colors
        assert_ne!(pal0[1], pal1[1]);
    }

    #[test]
    fn sprite_data_out_of_range_pet_falls_back_to_frog() {
        // pet >= 6 falls back to Frog (original sprites) via `_ =>` arm
        let (_, pal_out) = sprite_data(&SessionStatus::Idle, 0, 99);
        let (_, pal_frog) = sprite_data(&SessionStatus::Idle, 0, 0);
        assert_eq!(pal_out[1], pal_frog[1]);
    }
}
