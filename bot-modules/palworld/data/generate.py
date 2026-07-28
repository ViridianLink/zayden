import json, os, collections

OUT = "/home/oscar/Documents/Viridian/zayden/bot-modules/palworld/data"
os.makedirs(OUT, exist_ok=True)

def psp(n): return json.load(open(f"psp/data/json/{n}"))
def psp_en(n): return json.load(open(f"psp/data/json/l10n/en/{n}"))
def pwst(n): return json.load(open(f"pwst/resources/game_data/{n}"))

def r1(v): return round(float(v), 1)


MAP_AREAS = [
    ("tree",     347351.5,  689148.5, -818197.0, -476400.0),
    ("palpagos", -1099400.0, 349400.0, -724400.0,  724400.0),
]

def map_of(x, y):
    if x is None or y is None:
        return None
    for name, min_x, max_x, min_y, max_y in MAP_AREAS:
        if min_x <= x <= max_x and min_y <= y <= max_y:
            return name
    return None

def placed(entry):
    area = map_of(entry.get("x"), entry.get("y"))
    return entry if area is None else {**entry, "map": area}

def dump(name, obj):
    p = f"{OUT}/{name}"
    with open(p, "w") as f:
        json.dump(obj, f, separators=(",", ":"), sort_keys=False)
        f.write("\n")
    print(f"{name}: {len(obj)} entries, {os.path.getsize(p)} bytes")

# fast travel points: union of psp (class + coords) and pwst (11 newer)
a, b, l = psp("fast_travel_points.json"), pwst("fast_travel_points.json"), psp_en("fast_travel_points.json")
ftp = []
for gid in sorted(set(a) | set(b)):
    src = a.get(gid) or b[gid]
    name = (l.get(gid) or {}).get("localized_name") or b.get(gid, {}).get("localized_name") or src.get("id") or gid
    kind = "map_point" if a.get(gid, {}).get("class") == "BP_LevelObject_UnlockMapPoint_C" else "tower"
    ftp.append(placed({"id": gid, "name": name, "kind": kind, "x": r1(src["x"]), "y": r1(src["y"])}))
dump("fast_travel_points.json", ftp)
print("   by map:", dict(collections.Counter(e.get("map") for e in ftp)))

# bosses (NormalBossDefeatFlag is keyed by spawner_id)
bounty = pwst("boss_mapping.json")["boss_defeat_flag_map"]      # reward item -> spawner_id
spawner_reward = {v: k for k, v in bounty.items()}
pal_names = psp_en("pals.json")
bosses, seen = [], set()
for v in psp("bosses.json").values():
    sp = v["spawner_id"]
    if sp in seen:
        continue
    seen.add(sp)
    cid = v["character_id"]
    lower = cid.lower()
    base = cid
    for prefix in ("boss_", "predator_", "raid_", "summon_", "gym_"):
        if lower.startswith(prefix):
            base = cid[len(prefix):]
            break
    name = (pal_names.get(base) or pal_names.get(cid) or {}).get("localized_name")
    if not name or cid == "None":
        # psp records no character for these spawners; label them by locale.
        name = sp.replace("_", " ")
    bosses.append(placed({
        "spawner": sp,
        "character_id": cid,
        "name": name,
        "alpha": lower.startswith("boss_"),
        "bounty": sp in spawner_reward,
        "level": v.get("level", 0),
        "x": r1(v["x"]), "y": r1(v["y"]),
    }))
bosses.sort(key=lambda e: e["spawner"])
dump("bosses.json", bosses)
print("   alphas:", sum(1 for e in bosses if e["alpha"]),
      "bounty spawners:", sum(1 for e in bosses if e["bounty"]),
      "of", len(set(bounty.values())), "referenced")
print("   by map:", dict(collections.Counter(e.get("map") for e in bosses)))

# relics (capture_power == the Lifmunk effigies)
relics = [placed({"id": gid, "type": v["relic_type"], "x": r1(v["x"]), "y": r1(v["y"])})
          for gid, v in sorted(json.load(open("psp/data/json/relics.json")).items())]
# Present in a real save's RelicObtainForInstanceFlagByType but in neither
# upstream dump, so its position - and therefore its map - is unknown.
EXTRA_EFFIGIES = ["334DA75D43DC9E37A3C84B81A98BB0A8"]
known = {e["id"] for e in relics}
relics += [{"id": g, "type": "capture_power", "x": None, "y": None}
           for g in EXTRA_EFFIGIES if g not in known]
relics.sort(key=lambda e: e["id"])
dump("relics.json", relics)
print("   by type:", dict(collections.Counter(e["type"] for e in relics)))
print("   by map:", dict(collections.Counter(e.get("map") for e in relics)))

rt_names = psp_en("relics.json")
rt_data = psp("relic_data.json")
relic_types = [{"key": k, "name": (rt_names.get(k) or {}).get("localized_name", k),
                "max_rank": (rt_data.get(k) or {}).get("max_rank", 0)}
               for k in sorted({e["type"] for e in relics} | set(rt_data))]
dump("relic_types.json", relic_types)

# technologies
tn = psp_en("technologies.json")
techs = [{"id": k, "name": (tn.get(k) or {}).get("localized_name", k),
          "boss": bool(v.get("is_boss_technology")), "level": v.get("level_cap", 0),
          "cost": v.get("cost", 0)}
         for k, v in sorted(psp("technologies.json").items())]
EXTRA_TECHS = ["OverHeatRifle", "PalBox", "ShotgunBullet"]
known = {t["id"] for t in techs}
techs += [{"id": t, "name": t, "boss": False, "level": 0, "cost": 0}
          for t in EXTRA_TECHS if t not in known]
techs.sort(key=lambda t: t["id"])
dump("technologies.json", techs)
print("   boss techs:", sum(1 for t in techs if t["boss"]))

# missions
mn = psp_en("missions.json")
KIND = {"EPalQuestType::Main": "main", "EPalQuestType::Sub": "sub", "EPalQuestType::Hidden": "hidden"}
missions = [{"id": k, "name": (mn.get(k) or {}).get("localized_name", k),
             "kind": KIND.get(v.get("quest_type"), "other")}
            for k, v in sorted(psp("missions.json").items())]
dump("missions.json", missions)
print("   by kind:", dict(collections.Counter(m["kind"] for m in missions)))

# map areas (FindAreaFlagMap keys)
EXTRA_AREAS = ["BOSS_KingWhale", "SkyIsland02"]
dump("areas.json", sorted(set(pwst("world_map_areas.json")["areas"]) | set(EXTRA_AREAS)))

# Tower bosses
TOWERS = [
    ("GrassBoss",             "Zoe & Grizzbolt",     "Rayne Syndicate Tower",              "palpagos"),
    ("ForestBoss",            "Lily & Lyleen",       "Free Pal Alliance Tower",            "palpagos"),
    ("DesertBoss",            "Marcus & Faleris",    "Brothers of the Eternal Pyre Tower", "palpagos"),
    ("ElectricBoss",          "Axel & Orserk",       "PIDF Tower",                         "palpagos"),
    ("SnowBoss",              "Victor & Shadowbeak", "PIDF Tower, Astral Mountains",       "palpagos"),
    ("SakurajimaBoss",        "Saya & Selyne",       "Moonflower Tower, Sakurajima",       "palpagos"),
    ("VikingBoss",            "Bjorn & Bastigor",    "Bastion of Feybreak",                "palpagos"),
    ("SorajimaBoss",          "Sky Island tower",    "Sky Island",                         "palpagos"),
    ("KingWhaleBoss",         "King Whale",          "Ocean",                              "palpagos"),
    ("WorldTreeBoss",         "World Tree boss",         "Within the Seal",       "tree"),
    ("WorldTreeMiddleBoss1",  "World Tree guardian I",   "Rotmist Root",          "tree"),
    ("WorldTreeMiddleBoss2",  "World Tree guardian II",  "Shinespore Root",       "tree"),
    ("WorldTreeMiddleBoss3",  "World Tree guardian III", "Forbidden Laboratory",  "tree"),
]
gated = {v.get("require_defeat_tower_boss") for v in psp("technologies.json").values()}
gated.discard("EPalBossType::None")
assert gated <= {f"EPalBossType::{k}" for k, _, _, _ in TOWERS}, sorted(gated)
dump("towers.json", [{"id": k, "flag": f"BOSS_BATTLE_NAME_{k}", "name": n, "location": loc, "map": area}
                     for k, n, loc, area in TOWERS])

# paldeck: only real, deck-indexed Pals (excludes humans and raid monsters)
pal_src = psp("pals.json")
pals = [{"id": k, "name": (pal_names.get(k) or {}).get("localized_name", k),
         "dex": v["pal_deck_index"], "tribe": v.get("tribe", "")}
        for k, v in sorted(pal_src.items())
        if v.get("is_pal") and v.get("pal_deck_index", 0) > 0]
dump("pals.json", pals)
print("   unique dex numbers:", len({e["dex"] for e in pals}))
