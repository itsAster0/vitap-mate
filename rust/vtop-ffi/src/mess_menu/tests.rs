use super::*;

fn sample_rows() -> Vec<Vec<String>> {
    vec![
        vec![
            "VIT-AP MESS MENU FOR THE MONTH OF OCTOBER".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
        ],
        vec![
            "Day".into(),
            "Breakfast".into(),
            "Lunch".into(),
            "Snacks".into(),
            "Dinner".into(),
        ],
        vec![
            "Thu\n1, 15, 29".into(),
            "Idli".into(),
            "Aloo Mutter Curry (1), Chickpea Masala (15, 29)".into(),
            "Samosa (2)".into(),
            "Chicken Curry (Non-Veg)".into(),
        ],
        vec![
            "".into(),
            "Chutney".into(),
            "Rice".into(),
            "".into(),
            "Curd".into(),
        ],
        vec![
            "Fri 2, 16".into(),
            "Dosa".into(),
            "Dal".into(),
            "Tea".into(),
            "Paneer".into(),
        ],
        vec![
            "Instructions".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
        ],
        vec![
            "1. Carry your ID card\n2. No outside food".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
        ],
    ]
}

fn table(name: &str, rows: Vec<Vec<String>>) -> MessTable {
    MessTable {
        name: name.into(),
        rows,
    }
}

fn item(name: &str) -> MessItem {
    MessItem {
        name: name.into(),
        non_veg: false,
    }
}

fn non_veg_item(name: &str) -> MessItem {
    MessItem {
        name: name.into(),
        non_veg: true,
    }
}

fn item_names(sheet: &MessSheet, day: i32, meal: Meal) -> Vec<String> {
    sheet
        .items_for(day, meal)
        .into_iter()
        .map(|i| i.name)
        .collect()
}

fn block_of(sheet: &MessSheet, day: i32) -> Option<Vec<i32>> {
    sheet
        .days
        .iter()
        .find(|d| d.day == day)
        .map(|d| d.block.clone())
}

#[test]
fn parses_month_blocks_and_instructions() {
    let menu = parse_mess_menu(
        vec![
            table("Veg & Non-Veg ", sample_rows()),
            table("Notes", vec![vec!["nothing".into()]]),
        ],
        None,
    )
    .unwrap();
    assert_eq!(menu.sheets.len(), 1);
    let sheet = &menu.sheets[0];
    assert_eq!(sheet.name, "Veg & Non-Veg");
    assert_eq!(menu.month, Some(10));
    assert_eq!(block_of(sheet, 15), Some(vec![1, 15, 29]));
    assert_eq!(
        item_names(sheet, 29, Meal::Breakfast),
        vec!["Idli", "Chutney"]
    );
    assert_eq!(
        sheet.instructions,
        vec!["Carry your ID card", "No outside food"]
    );
    assert_eq!(item_names(sheet, 16, Meal::Breakfast), vec!["Dosa"]);
}

#[test]
fn resolves_date_variants_quantities_and_non_veg() {
    let sheet = parse_mess_menu(vec![table("M", sample_rows())], None)
        .unwrap()
        .sheets
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(
        sheet.items_for(1, Meal::Lunch),
        vec![item("Aloo Mutter Curry"), item("Rice")]
    );
    assert_eq!(
        sheet.items_for(15, Meal::Lunch),
        vec![item("Chickpea Masala"), item("Rice")]
    );
    assert_eq!(sheet.items_for(1, Meal::Snacks), vec![item("Samosa (2)")]);
    assert_eq!(
        sheet.items_for(1, Meal::Dinner)[0],
        non_veg_item("Chicken Curry")
    );
    assert_eq!(
        resolve_mess_item("Kheer (1)".into(), 15, vec![1, 15, 29]),
        None
    );
}

#[test]
fn refuses_other_spreadsheets() {
    let err =
        parse_mess_menu(vec![table("S", vec![vec!["a".into(), "b".into()]])], None).unwrap_err();
    assert_eq!(
        err,
        MessMenuError::Format {
            reason: "This spreadsheet doesn't look like a mess menu.".into()
        }
    );
}

#[test]
fn meal_windows_round_trip_and_fall_back() {
    let defaults = default_meal_windows();
    assert_eq!(
        encode_meal_windows(defaults.clone()),
        "435-540,750-840,990-1095,1155-1260"
    );
    let mut custom = defaults.clone();
    custom[1] = MealWindow {
        start: 700,
        end: 800,
    };
    assert_eq!(
        decode_meal_windows(Some(encode_meal_windows(custom.clone()))),
        custom
    );
    assert_eq!(decode_meal_windows(Some("junk".into())), defaults);
    assert_eq!(decode_meal_windows(None), defaults);
}

#[test]
fn current_or_next_meal_and_year() {
    let defaults = default_meal_windows();
    assert_eq!(
        current_or_next_meal(0, defaults.clone()),
        Some(Meal::Breakfast)
    );
    assert_eq!(
        current_or_next_meal(9 * 60, defaults.clone()),
        Some(Meal::Lunch)
    );
    assert_eq!(current_or_next_meal(21 * 60, defaults), None);
    assert_eq!(
        mess_menu_year(Some(10), "Menu October 2025".into(), 2026, 11),
        2025
    );
    assert_eq!(mess_menu_year(Some(1), "Menu".into(), 2026, 11), 2027);
    assert_eq!(mess_menu_year(Some(12), "Menu".into(), 2026, 2), 2025);
    assert_eq!(mess_menu_year(None, "Menu".into(), 2026, 2), 2026);
}

#[test]
fn weekday_decides_conflicting_dates_and_fills_the_fortnight() {
    // November 2026 starts on a Sunday: 10 and 24 are Tuesdays, 25 a Wednesday, 26 a Thursday.
    let rows = vec![
        vec![
            "MESS MENU FOR THE MONTH OF NOVEMBER".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
        ],
        vec![
            "Day".into(),
            "Breakfast".into(),
            "Lunch".into(),
            "Snacks".into(),
            "Dinner".into(),
        ],
        vec![
            "Tue\n10, 25".into(),
            "Dosa".into(),
            "".into(),
            "".into(),
            "".into(),
        ],
        vec![
            "Wed / 11, 26".into(),
            "Vada".into(),
            "".into(),
            "".into(),
            "".into(),
        ],
        vec![
            "Thu      12, 26".into(),
            "Poori".into(),
            "".into(),
            "".into(),
            "".into(),
        ],
    ];
    let sheet = parse_mess_menu(vec![table("M", rows.clone())], Some(2026))
        .unwrap()
        .sheets
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(item_names(&sheet, 24, Meal::Breakfast), vec!["Dosa"]);
    assert_eq!(item_names(&sheet, 25, Meal::Breakfast), vec!["Vada"]);
    assert_eq!(item_names(&sheet, 26, Meal::Breakfast), vec!["Poori"]);
    assert_eq!(block_of(&sheet, 24), Some(vec![10, 24]));
    // Without the year, the dates stay as listed and the first block keeps a duplicate.
    let raw = parse_mess_menu(vec![table("M", rows)], None)
        .unwrap()
        .sheets
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(item_names(&raw, 25, Meal::Breakfast), vec!["Dosa"]);
    assert_eq!(item_names(&raw, 26, Meal::Breakfast), vec!["Vada"]);
}

#[derive(serde::Deserialize)]
struct RawTable {
    name: String,
    rows: Vec<Vec<String>>,
}

#[test]
fn reads_the_november_2026_menu() {
    let tables: Vec<MessTable> = serde_json::from_str::<Vec<RawTable>>(NOV_2026_TABLES)
        .unwrap()
        .into_iter()
        .map(|t| MessTable {
            name: t.name,
            rows: t.rows,
        })
        .collect();
    let menu = parse_mess_menu(tables, Some(2026)).unwrap();
    let names: Vec<&str> = menu.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["Veg & Non-Veg", "Special"]);
    assert_eq!(menu.month, Some(11));
    let sheet = &menu.sheets[0];
    // Every date of the month has exactly one block.
    let day_keys: Vec<i32> = sheet.days.iter().map(|d| d.day).collect();
    assert_eq!(day_keys, (1..=30).collect::<Vec<_>>());
    assert_eq!(block_of(sheet, 8), Some(vec![8, 22]));
    let breakfast8 = sheet.items_for(8, Meal::Breakfast);
    assert_eq!(breakfast8.first().unwrap().name, "Shavige Bath");
    assert_eq!(breakfast8.last().unwrap(), &non_veg_item("Egg Bhurji"));
    assert_eq!(
        sheet.items_for(8, Meal::Snacks).first().unwrap().name,
        "Dahi Papdi Chat"
    );
    assert_eq!(
        sheet.items_for(24, Meal::Breakfast).first().unwrap().name,
        "Ghee Podi Dosa (2 Pcs Thin)"
    );
    assert_eq!(
        sheet.items_for(25, Meal::Breakfast).first().unwrap().name,
        "Methu Vada (Big size 3pcs, if not 4pcs)"
    );
    assert_eq!(
        sheet.items_for(26, Meal::Breakfast).first().unwrap().name,
        "Methi Poori (3 Pcs standard size)"
    );
    assert_eq!(
        sheet.items_for(3, Meal::Breakfast).first().unwrap().name,
        "Onion Dosa - 2 Pcs Std Size"
    );
    assert_eq!(
        sheet.items_for(17, Meal::Breakfast).first().unwrap().name,
        "Masala Dosa - 2 Pcs Std Size"
    );
    let paneer = sheet
        .items_for(2, Meal::Dinner)
        .into_iter()
        .find(|i| i.name.contains("Paneer"))
        .unwrap();
    assert_eq!(paneer, item("Paneer Butter Masala"));
    assert!(
        sheet
            .instructions
            .first()
            .unwrap()
            .starts_with("Thick curd must be served")
    );
}

const NOV_2026_TABLES: &str = r###"[{"name":"Veg & Non-Veg","rows":[["","","","",""],["VEG & NON-VEG MESS MENU FOR THE MONTH OF NOVEMBER","","","",""],["Day","Breakfast","Lunch","Snacks","Dinner"],["Sun\n1, 15, 29","Shavige Bath","Onions & Lemon Salad","Cup Cake (Egg Less) 2 Pcs (1), Dahi Puri (8 Pcs), Onions (15, 29)","Beetroot & Carrot Salad"],["","Paneer Paratha - 2 Pcs Thin","Chapathi","Masala Tea/Coffee/Milk","Pulka"],["","Peanut Chutney","White Rice","","White Rice"],["","Thick Curd, Mango Pickle","Dal Fry","","Pesara Pappu"],["","Brown Bread+butter+Jam","Chicken Dum Biryani/Paneer Dum Biryani","","Radish Sambar"],["","Sprouts (1 small cup)","Chicken Thick Gravy / Hyderabadi Mirchi Ka Salan","","Ragi Idly, Groundnut Chutney"],["","Tea/Coffee/Milk","Tomato Rasam","","Dondakaya Stir Fry"],["","","Onion Raitha","","Vegetable Kurma Gravy"],["","","Triangular Fryums","","Thick Curd"],["","Egg Bhurji (Non-Veg)","Mint Lemon Jal Jeera Drink (200 ml)","","Papaya Cut Fruit"],["","","Fresh Gongura Chutney","","Tomato Pickle"],["","","Basundhi (60 ml)","","Milk + Cofee Powder"],["","","","",""],["Mon\n2, 16, 30","Ghee + Veg Pongal","Boiled Chana Salad with Onion & Tomato","Dry Maggi (Non-Sticky)","Onions & Lemon Salad"],["","Bhature - 2 Pcs Big ","Chapathi","Tomato Sauce","Roti"],["","Groundnut Chutney, Sambar","White Rice","Tea/Coffee/Milk","White Rice"],["","Chole Curry","Palak Dal","","Pesara Pappu"],["","White Bread+Butter+Jam","Brinjal Sambar","","Andhra Charu"],["","Sprouts (1 small cup)","Tomato Rice","","Bhagara Rice"],["","Tea/Coffee/Milk","Pesara Punugulu Curry (Pakka Andhra Style)","","Hyderabadi Chicken Curry (Non-Veg)"],["","","Capsicum Tomato Masala","","Paneer Butter Masala (Veg)"],["","","Fryums","","Ladies Finger Boiled Fry"],["","Boiled Egg (Non-Veg)","Butter Milk","","Thick Curd"],["","","Sorakaya Perugu Chutney","","Water Melon Cut Fruit "],["","","Jilebi","","Radish Fresh Chutney"],["","","Ghee + Podi","","Milk + Cofee Powder"],["","","","",""],["Tue\n3, 17","Onion Dosa (3), Masala Dosa (17) - 2 Pcs Std Size","Carrot & Cucumber Salad","Aloo Samosa 1 Big Pc","Beetroot & Carrot Salad"],["","Vada Pav","Roti","Tomato Sauce, Salt Mirchi","Pulka"],["","Coconut Chutney","White Rice","Ginger Tea/Coffee/Milk","White Rice"],["","Mint Chutney","Dosakaya Pappu","","Dal Makhani"],["","Brown Bread+Butter+Jam","Beetroot Tomato Rasam","","Kadamba Sambar"],["","Ragi Malt with Salt (1 Medium Glass)","Bisbele Bath","","Mushroom Biryani"],["","Tea/Coffee/Milk","Kakarakaya (Bitter Gourd) Fry","","Cluster Beans Coconut Fry"],["","","Rajma Masala","","Aloo Mutter Curry"],["","","Potato Chips","","Thick Curd"],["","Boiled Egg (Non-Veg)","Dahi Vada","","Banana Fruit"],["","","Dosakaya Tomato Chutney","","Ginger Pickle"],["","","Ghee + Podi","","Milk + Cofee Powder"],["","","","",""],["Wed      4, 18","Pesarattu Thin 2 Pcs Std Size","Onions & Lemon Salad","Bhelpuri","Beetroot & Carrot Salad"],["","Tomato Suji Upma","Palak Roti","Ginger Tea/Coffee/Milk","Chapathi"],["","Coconut Chutney","White Rice","","White Rice"],["","Ginger Chutney","Amaranthus Dal","","Tomato Dal"],["","White Bread+Butter+Jam","Pachi Pulusu","","Garlic Rasam"],["","Sprouts (1 small cup)","Bhagara Rice","","Broken Wheat Rava Upma, Coconut Chutney"],["","Tea/Coffee/Milk","Kadai Chicken","","Bottle Guard Chana Dal Fry"],["","","Kadai Paneer","","Aloo Mutter Curry"],["","","Vegetable Jalfrezi","","Thick Curd"],["","Masala Onion Omlet  (Non-Veg)","Papad","","Pine Apple Cut Fruit "],["","","Butter Milk","","Lemon Pickle"],["","","Fresh Vegetable Chutney","","Milk + Cofee Powder"],["","","Semiya Sabhudhana Kheer (4), Double ka Meeta (18)","",""],["Thu\n5, 19","Medu Vada (Big size 3pcs, if not 4pcs)","Carrot & Cucumber Salad","Mysore Bonda 2 Pcs Std Size (Less Oily)","Beetroot & Carrot Salad"],["","Pav Bhaji","Pulka","Coconut Chutney","Roti"],["","Sambar","White Rice","Tea/Coffee/Milk","White Rice"],["","Coconut Chutney","Ridge Gourd Dal","","Dal Fry"],["","Brown Bread+Butter+Jam","Drumstick Brinjal Sambar","","Beetroot Tomato Rasam"],["","Sprouts (1 small cup)","Pudina Rice","","Veg Manchurian, Noodles, Tomato Sauce"],["","Tea/Coffee/Milk","Guthi Vankaya Curry","","Dhaba Style Egg Curry (Non-Veg)"],["","","Kala Chana Masala","","Rajma Masala"],["","","Finger Fryums","","Thick Curd"],["","","Lemon Water + Sabja Seeds","","Green Grapes"],["","","Potlakaya Perugu Chutney","","Gongura Fresh Chutney"],["","","Ghee + Podi","","Milk + Cofee Powder"],["","","","",""],["","","","",""],["Fri\n6, 20","Poori (3 pcs standard size)","Beetroot & Cucumber Salad","Sambar Vada 2 Pcs (Separate)","Onions & Lemon Salad"],["","Rice Rava Upma (Dry)","Chapathi","Masala Tea/Coffee/Milk","Beetroot Chapathi "],["","Aloo Basin Chutney","White Rice","","White Rice"],["","Dal Chutney","Dal Tadka","","Dal"],["","Brown Bread+Butter+Jam","Andhra Charu","","Mix Veg Sambar"],["","Ragi Malt with Salt (1 Medium Glass)","Jeera Rice","","Chicken Tikka Masala (Non-Veg)"],["","Tea/Coffee/Milk","Raw Banana Fry","","Paneer Lababdar (Veg)"],["","","Turnip Spinach Curry (Punjabi Style)","","Amritsari Chole"],["","","Papad","","Thick Curd"],["","Boiled Egg (Non-Veg)","Lassi","","Muskmelon Cut Fruit"],["","","Beerakaya Chutney","","Amla Pickle"],["","","Carrot Halwa","","Milk + Cofee Powder"],["","","","",""],["Sat\n7, 21","Uggani + Mirchi Bajji ","Carrot & Cucumber Salad","Vada Pav 2 Pcs ","Beetroot & Carrot Salad"],["","Moong Dal Palak Tepla (Hand Made) 2 Pcs","Pulka","Mint Sauce ","Roti"],["","Groundnut Chutney","White Rice","Cold Badam Milk (1 Medium Glass)","White Rice"],["","Onion Tomato Chutney","Mudda Pappu","","Dal Maharani"],["","Brown Bread+Butter+Jam","Pachi Pulusu","","Drumstick Sambar"],["","Ragi Malt with Salt (1 Medium Glass)","Pulihora (Tamarind Rice)","","Ghee + Podi + Idly, Coconut Chutney"],["","Tea/Coffee/Milk","Cauli Flower Fry","","Tomato Baingan Fry "],["","","Chole Curry","","Thick Curd"],["","","Papad","","Banana"],["","Boiled Egg (Non-Veg)","Majjiga Pulusu","","Red Chilli Pickle"],["","","Avakaya (Mango Pickle)","","Milk + Cofee Powder"],["","","","",""],["","","Ghee + Podi","",""],["Sun\n8, 22","Shavige Bath","Onions & Lemon Salad","Dahi Papdi Chat","Beetroot & Carrot Salad"],["","Aloo Paratha (2 Pcs Thin)","Chapathi","Onions","Pulka"],["","Mint Coriander Chutney","White Rice","Tea/Coffee/Milk","White Rice"],["","Thick Curd, Mango Pickle","Dal Makhani","","Pesara Pappu"],["","White Bread+Butter+Jam","Chicken Fry Piece Biryani/Paneer Fry Piece Biryani","","Radish Sambar"],["","Sprouts (1 small cup)","Chicken Thick Gravy / Hyderabadi Mirchi Ka Salan","","Karam Dosa (2 Pcs Thin), Coconut Chutney"],["","Tea/Coffee/Milk","Onion Raitha","","Dondakaya Fry"],["","","Tomato Rasam","","Drumstick Tomato Curry"],["","","Fryums","","Thick Curd"],["","Egg Bhurji (Non-Veg)","Mint Lemon Jal Jeera Drink (200 ml)","","Papaya Cut Fruit"],["","","Fresh Gongura Chutney","","Tomato Pickle"],["","","Vanilla Ice Cream  (Not Frozen Dessert)","","Milk + Cofee Powder"],["","","","",""],["Mon\n9, 23","Vegetable Moong Dal Kitchidi","Boiled Chana Salad with Onion & Tomato","Mexican Sweet Corn Salad","Onions & Lemon Salad"],["","Bhature - 3 Pcs Big","Chapathi","Lemon Sauce","Methi Roti"],["","Peanut Chutney, Sambar","White Rice","Masala Tea/Coffee/Milk","White Rice"],["","Chole Curry","Palak Dal","","Dal Tadka"],["","Brown Bread+Butter+Jam","Brinjal Sambar","","Jeera-Milagu Rasam"],["","Sprouts (1 small cup)","Ghee Rice","","Bhagara Rice"],["","Tea/Coffee/Milk","French Beans Potato Fry","","Chettinad Chicken Curry (Non-Veg)"],["","","Ghobi Masala","","Paneer Green Peas Masala (Veg)"],["","","Fryums","","Snake Gourd Poriyal"],["","Masala Omlet (Non-Veg)","Butter Milk","","Thick Curd"],["","","Sorakaya Perugu Chutney, Ghee + Podi","","Water Melon Cut Fruit "],["","","Semiya Sabhudhana Kheer","","Gongura Pickle"],["","","Ghee + Podi","","Milk + Cofee Powder"],["","","","",""],["Tue\n10, 25","Ghee Podi Dosa (2 Pcs Thin)","Carrot & Cucumber Salad","Aloo Samosa 1 Big Pc","Beetroot & Carrot Salad"],["","Pav Bhaji","Roti","Tomato Sauce, Salt Mirchi","Pulka"],["","Coconut Chutney, Sambar","White Rice","Tea/Coffee/Milk","White Rice"],["","Mint Chutney","Amaranthus Dal","","Dal Makhani"],["","White Bread+Butter+Jam","Beetroot Tomato Rasam","","Kadamba Sambar"],["","Ragi Malt with Salt (1 Medium Glass)","Bisbele Bath","","Mushroom Biryani"],["","Tea/Coffee/Milk","Kakarakaya (Bitter Gourd) Fry","","French Beans Coconut Poriyal"],["","","Rajma Masala","","Aloo Mutter Curry"],["","","Kara Boondi","","Thick Curd"],["","Scrambled Egg (Non-Veg)","Dahi Vada","","Banana"],["","","Dosakaya Tomato Chutney","","Tomato Pickle"],["","","","","Milk + Cofee Powder"],["","","Ghee + Podi","",""],["Wed\n11, 26","Methu Vada (Big size 3pcs, if not 4pcs)","Onions & Lemon Salad","Bandi Style Ragda Chat","Beetroot & Carrot Salad"],["","Mix Veg Suji Bun 2 Pcs","Palak Roti","Onion Small pieces","Chapathi"],["","Sambar","White Rice","Ginger Tea/Coffee/Milk","White Rice"],["","Coconut Chutney","Gongura (Sorrel Leaves) Dal","","Tomato Dal"],["","Brown Bread+Butter+Jam","Rasam","","Drumstick Sambar"],["","Sprouts (1 small cup)","Vegetable Dum Pulav","","Sooji Upma, Coconut Chutney"],["","Tea/Coffee/Milk","Andhra Chicken Fry","","Bottle Guard Tomato Stir Fry"],["","","Kaju Tomato Paneer Fry","","Soya Chunks Masala"],["","","Red Beans Curry","","Thick Curd"],["","Boiled Egg (Non-Veg)","Trinagle Fryums","","Green Grapes"],["","","Butter Milk","","Lemon Pickle"],["","","Coriander + Tomato Chutney","","Milk + Cofee Powder"],["","","Gulab Jamun","",""],["Thu\n12, 26","Methi Poori (3 Pcs standard size)","Carrot & Cucumber Salad","Boiled Peanuts with Onion/Tomato(12 ) ","Onions & Lemon Salad"],["","Lemon Sevai","Pulka","Boiled Black Chana with Onion/Tomato (26)","Roti"],["","Aloo Mutter Curry (12), Chickpea Masala (26)","White Rice","Tea/Coffee/Milk","White Rice"],["","Coconut Chutney ","Sorrel Leaves Dal (Gongura Pappu)","","Dal Fry"],["","White Bread+Butter+Jam","Drumstick Brinjal Sambar","","Beetroot Tomato Rasam"],["","Sprouts (1 small cup)","Pudina Rice","","Veg Manchuria, Noodles, Tomato Sauce"],["","Tea/Coffee/Milk","Guthi Vankaya Curry","",""],["","","Ridge Gourd Fry","","Black Channa Curry"],["","","Finger Fryums","","Thick Curd"],["","Masala Onion Omlet (Non-Veg)","Lemon Water + Sabja Seeds","","Banana"],["","","Potlakaya Perugu Chutney","","Gongura Fresh Chutney"],["","","","","Milk + Cofee Powder"],["","","Ghee + Podi","",""],["Fri\n13, 27","Uggani + Mirchi Bajji","Beetroot & Cucumber Salad","Aloo Bonda (Medium size 2 pc)","Onions & Lemon Salad"],["","Vegetable Daliya Kitchidi","Chapathi","Coconut Chutney","Lacha Paratha "],["","Coconut Chutney","White Rice","Tea/Coffee/Milk","White Rice"],["","Mango Pickle","Dal Tadka","","Pesara Pappu"],["","Brown Bread+Butter+Jam","Rasam","","Mix Veg Sambar"],["","Sprouts (1 small cup)","Jeera Rice","","Garlic Chicken Curry (Non-Veg)"],["","Tea/Coffee/Milk","Raw Banana Fry","","Paneer Tikka Masala (Veg)"],["","","Red Rajma Masala","","Tomato Peas Capsicum Masala"],["","","Papad","","Thick Curd"],["","Boiled Egg (Non-Veg)","Butter Milk","","Muskmelon Cut Fruit"],["","","Beerakaya Chutney","","Mango Pickle"],["","","Moong Dal Payasam","","Milk + Cofee Powder"],["","","","",""],["Sat\n14, 28","Mix Veg Utappam - 2 Pcs Std Size","Carrot & Cucumber Salad","Punugulu 8 Pcs Medium Size","Beetroot & Carrot Salad"],["","Vegetable Poha","Pulka","Groundnut/Coconut Chutney","Roti"],["","Peanut Chutney","White Rice","Cold Badam Milk 150 ml","White Rice"],["","Tomato Chutney","Mudda Pappu","","Dal Maharani"],["","White Bread+Butter+Jam","Pachi Pulusu","","Bendakaya (Bhindi) Pulusu"],["","Sprouts (1 small cup)","Pulihora (Tamarind Rice)","","Vegetable Fried Rice, Gobhi Manchuria, Tomato Sauce"],["","Tea/Coffee/Milk","Radish Tomato Stir Fry","","Cabbage Beans Poriyal"],["","","Chole Curry","","Thick Curd"],["","","Papad","","Fruit Custard with Custard Apple"],["","Boiled Egg (Non-Veg)","Majiga Pulusu","","Red Chilli Pickle"],["","","Avakaya (Mango Pickle)","","Milk + Cofee Powder"],["","","","",""],["","","Ghee + Podi","",""],["","","","",""],["","MESS SERVICE INSTRUCTIONS","","",""],["","1. Thick curd must be served as per the menu, either during lunch or dinner.\n2. Fresh salad must be served every day as per the menu.\n3. The toaster should be kept functional every day.\n4. A weighing machine must be used while serving Chicken and Paneer and it must be functionally available in the mess area everyday.\n5. Chicken should weigh 150 g (and 180 g on Sundays) after cooking, i.e., before serving (excluding bowl weight and gravy). Fish should weight 120 g after cooking (excluding bowl weight)\n6. Paneer/Mushroom should be soft and must weigh 75 g/50 g after cooking, i.e., before serving (excluding bowl weight and gravy).\n7. Roti/Phulka/Chapathi should be prepared using 100% good-quality Atta.\n8. White Rice (Sona Masuri) must be cooked properly every day — neither undercooked nor overcooked.\n9. All wet gravy-based curries should be prepared with 75% vegetables and 25% gravy.\n10. Egg Bhurji should be served as 60 g (after cooking).\n11. The quantity of an item may vary depending on the size served by the caterer.\n12. Fresh juice served in the Special Menu must be atleast 250 ml. Fresh juices, sweet items, and soups included in the Special Menu must be of the highest quality. Please use minimal water/milk while preparing the juices. \n13. Food items like Rava bonda , Vada should be served in 3 standard-sized pieces; if the size is smaller, 4 pieces must be served. (in breakfast)\n14. Ice Cream served during the Sunday lunch should be made primarily with milk and cream. It should not be frozen dessert type.","","",""]]},{"name":"Special","rows":[["","","","",""],["SPECIAL MESS MENU FOR THE MONTH OF NOVEMBER","","","",""],["Day","Breakfast","Lunch","Snacks","Dinner"],["Sun\n1, 15, 29","Shavige Bath","Onions & Lemon Salad","Cup Cake (Egg Less) 2 Pcs (1), Dahi Puri (8 Pcs), Onions (15, 29)","Beetroot & Carrot Salad"],["","Paneer Paratha - 2 Pcs Thin","Chapathi","Masala Tea/Coffee/Milk","Pulka"],["","Peanut Chutney","White Rice","","White Rice"],["","Thick Curd, Mango Pickle","Dal Fry","","Pesara Pappu"],["","Brown Bread+Peanut Butter+Jam","Chicken Dum Biryani/Paneer Dum Biryani","","Radish Sambar"],["","Sprouts (1 small cup)","Chicken Thick Gravy / Hyderabadi Mirchi Ka Salan","","Ragi Idly, Groundnut Chutney"],["","Tea/Coffee/Milk","Tomato Rasam","","Dondakaya Stir Fry"],["","Apple + Pomegranate Fresh Juice without Sugar","Onion Raitha","","Vegetable Kurma Gravy"],["","Cornflakes","Triangular Fryums","","Thick Curd"],["","Egg Bhurji (Non-Veg)","Mint Lemon Jal Jeera Drink (200 ml)","","Papaya Cut Fruit"],["","Raagi Malt (Veg)","Fresh Gongura Chutney","","Tomato Pickle"],["","","Vanilla Ice Cream  (Not Frozen Dessert)","","Milk + Cofee Powder"],["","","Gulab Jamun","","Lemon Coriander Soup"],["Mon\n2, 16, 30","Ghee + Veg Pongal","Boiled Chana Salad with Onion & Tomato","Dry Maggi (Non-Sticky)","Onions & Lemon Salad"],["","Bhature - 2 Pcs Big ","Chapathi","Tomato Sauce","Butter Methi Naan"],["","Groundnut Chutney, Sambar","White Rice","Tea/Coffee/Milk","White Rice"],["","Chole Curry","Palak Dal","","Pesara Pappu"],["","White Bread+Butter+Jam","Brinjal Sambar","","Andhra Charu"],["","Sprouts (1 small cup)","Tomato Rice","","Bhagara Rice"],["","Tea/Coffee/Milk","Pesara Punugulu Curry (Pakka Andhra Style)","","Hyderabadi Chicken Curry (Non-Veg)"],["","Banana Milk Shake without Sugar","Capsicum Tomato Masala","","Paneer Butter Masala (Veg)"],["","Cornflakes","Fryums","","Ladies Finger Boiled Fry"],["","Boiled Egg (Non-Veg)","Butter Milk","","Thick Curd"],["","Boiled Mixed Beans Salad (Veg)","Sorakaya Perugu Chutney","","Water Melon Cut Fruit "],["","","Ghee + Podi","","Radish Fresh Chutney"],["","","Jilebi","","Milk + Cofee Powder"],["","","","","Sweet Corn Soup"],["Tue\n3, 17","Onion Dosa (3), Masala Dosa (17) - 2 Pcs Std Size","Carrot & Cucumber Salad","Aloo Samosa 1 Big Pc","Beetroot & Carrot Salad"],["","Vada Pav","Roti","Tomato Sauce, Salt Mirchi","Pulka"],["","Coconut Chutney","White Rice","Ginger Tea/Coffee/Milk","White Rice"],["","Mint Chutney","Dosakaya Pappu","","Dal Makhani"],["","Brown Bread+Peanut Butter+Jam","Beetroot Tomato Rasam","","Kadamba Sambar"],["","Ragi Malt with Salt (1 Medium Glass)","Bisbele Bath","","Mushroom Biryani"],["","Tea/Coffee/Milk","Kakarakaya (Bitter Gourd) Fry","","Cluster Beans Coconut Fry"],["","Pomegranate Fresh Juice without Sugar","Rajma Masala","","Aloo Mutter Curry"],["","Chocos","Potato Chips","","Thick Curd"],["","Boiled Egg (Non-Veg)","Dahi Vada","","Banana Fruit"],["","Crunchy Soya Vegetable Salad (Veg)","Dosakaya Tomato Chutney","","Ginger Pickle"],["","","Ghee + Podi","","Milk + Cofee Powder"],["","","Makkan Peda","","Brocolli Soup"],["Wed           4, 18","Pesarattu Thin 2 Pcs Std Size","Onions & Lemon Salad","Bhelpuri","Beetroot & Carrot Salad"],["","Tomato Suji Upma","Palak Roti","Ginger Tea/Coffee/Milk","Chapathi"],["","Coconut Chutney","White Rice","","White Rice"],["","Ginger Chutney","Amaranthus Dal","","Tomato Dal"],["","White Bread+Butter+Jam","Pachi Pulusu","","Garlic Rasam"],["","Sprouts (1 small cup)","Bhagara Rice","","Broken Wheat Rava Upma, Coconut Chutney"],["","Tea/Coffee/Milk","Kadai Chicken","","Bottle Guard Chana Dal Fry"],["","Pineapple Fresh Juice without Sugar","Kadai Paneer","","Aloo Mutter Curry"],["","Chocos","Vegetable Jalfrezi","","Thick Curd"],["","Masala Onion Omlet  (Non-Veg)","Papad","","Pine Apple Cut Fruit "],["","Greek Yogurt Salad (Veg)","Butter Milk","","Lemon Pickle"],["","","Fresh Vegetable Chutney","","Milk + Cofee Powder"],["","","Semiya Sabhudhana Kheer (4), Double ka Meeta (18)","","Tomato Soup with Roasted Bread"],["Thu\n5, 19","Medu Vada (Big size 3pcs, if not 4pcs)","Carrot & Cucumber Salad","Mysore Bonda 2 Pcs Std Size (Less Oily)","Beetroot & Carrot Salad"],["","Pav Bhaji","Pulka","Coconut Chutney","Roti"],["","Sambar","White Rice","Tea/Coffee/Milk","White Rice"],["","Coconut Chutney","Ridge Gourd Dal","","Dal Fry"],["","Brown Bread+Butter+Jam","Drumstick Brinjal Sambar","","Beetroot Tomato Rasam"],["","Sprouts (1 small cup)","Pudina Rice","","Veg Manchurian, Noodles, Tomato Sauce"],["","Tea/Coffee/Milk","Guthi Vankaya Curry","","Chicken Tikka Fry/Malabar Fish Fry (NV), Paneer Tikka/Chilli Babycorn (Veg)"],["","Papaya Fresh Juice without Sugar","Kala Chana Masala","","Dhaba Style Egg Curry (Special Non-Veg), Paneer Bhurji (Special Veg)"],["","Cornflakes","Finger Fryums","","Rajma Masala"],["","","Lemon Water + Sabja Seeds","","Thick Curd"],["","","Potlakaya Perugu Chutney","","Green Grapes"],["","","Ghee + Podi","","Gongura Fresh Chutney"],["","","Mothichoor Laddu","","Milk + Cofee Powder"],["","","","","Creamy Mushroom Soup"],["Fri\n6, 20","Poori (3 pcs standard size)","Beetroot & Cucumber Salad","Sambar Vada 2 Pcs (Separate)","Onions & Lemon Salad"],["","Rice Rava Upma (Dry)","Chapathi","Masala Tea/Coffee/Milk","Beetroot Chapathi"],["","Aloo Basin Chutney","White Rice","","White Rice"],["","Dal Chutney","Dal Tadka","","Dal"],["","Brown Bread+Peanut Butter+Jam","Andhra Charu","","Mix Veg Sambar"],["","Ragi Malt with Salt (1 Medium Glass)","Jeera Rice","","Chicken Tikka Masala (Non-Veg)"],["","Tea/Coffee/Milk","Raw Banana Fry","","Paneer Lababdar (Veg)"],["","Grapes Fresh Juice with Light Sugar","Turnip Spinach Curry (Punjabi Style)","","Amritsari Chole"],["","Museli","Papad","","Thick Curd"],["","Boiled Egg (Non-Veg)","Lassi","","Muskmelon Cut Fruit"],["","Boiled Chick Peas Salad (Veg)","Beerakaya Chutney","","Amla Pickle"],["","","Carrot Halwa","","Milk + Cofee Powder"],["","","","","Mix Veg Raagi Soup"],["Sat\n7, 21","Uggani + Mirchi Bajji ","Carrot & Cucumber Salad","Vada Pav 2 Pcs ","Beetroot & Carrot Salad"],["","Moong Dal Palak Tepla (Hand Made) 2 Pcs","Pulka","Mint Sauce ","Roti"],["","Groundnut Chutney","White Rice","Cold Badam Milk 150 ml","White Rice"],["","Onion Tomato Chutney","Mudda Pappu","","Dal Maharani"],["","Brown Bread+Peanut Butter+Jam","Pachi Pulusu","","Drumstick Sambar"],["","Ragi Malt with Salt (1 Medium Glass)","Pulihora (Tamarind Rice)","","Ghee + Podi + Idly, Coconut Chutney"],["","Tea/Coffee/Milk","Cauli Flower Tomato Stir Fry","","Tomato Baingan Fry "],["","Mosambi Fresh Juice without Sugar","Chole Curry","","Thick Curd"],["","Cornflakes","Papad","","Banana"],["","Boiled Egg (Non-Veg)","Majjiga Pulusu","","Red Chilli Pickle"],["","Boiled Peanut Sundal (Veg)","Avakaya (Mango Pickle)","","Milk + Cofee Powder"],["","","Ghee + Podi","","Veg Manchow Soup"],["","","Chocolate Cake (Egg Less)","",""],["Sun\n8, 22","Shavige Bath","Onions & Lemon Salad","Dahi Papdi Chat","Beetroot & Carrot Salad"],["","Aloo Paratha (2 Pcs Thin)","Chapathi","Onions","Pulka"],["","Mint Coriander Chutney","White Rice","Tea/Coffee/Milk","White Rice"],["","Thick Curd, Mango Pickle","Dal Makhani","","Pesara Pappu"],["","White Bread+Butter+Jam","Chicken Fry Piece Biryani/Paneer Fry Piece Biryani","","Radish Sambar"],["","Sprouts (1 small cup)","Chicken Thick Gravy / Hyderabadi Mirchi Ka Salan","","Karam Dosa (2 Pcs Thin), Coconut Chutney"],["","Tea/Coffee/Milk","Onion Raitha","","Dondakaya Fry"],["","Apple + Pomegranate Fresh Juice without Sugar","Tomato Rasam","","Drumstick Tomato Curry"],["","Museli","Fryums","","Thick Curd"],["","Egg Bhurji (Non-Veg)","Mint Lemon Jal Jeera Drink (200 ml)","","Papaya Cut Fruit"],["","Ragi Malt (Veg)","Fresh Gongura Chutney","","Tomato Pickle"],["","","Vanilla Ice Cream  (Not Frozen Dessert)","","Milk + Cofee Powder"],["","","Gajar Ka Halwa","","Lemon Coriander Soup"],["Mon\n9, 23","Vegetable Moong Dal Kitchidi","Boiled Chana Salad with Onion & Tomato","Mexican Sweet Corn Salad","Onions & Lemon Salad"],["","Bhature - 3 Pcs Big","Chapathi","Lemon Sauce","Butter Methi Naan"],["","Peanut Chutney, Sambar","White Rice","Masala Tea/Coffee/Milk","White Rice"],["","Chole Curry","Palak Dal","","Dal Tadka"],["","Brown Bread+Peanut Butter+Jam","Brinjal Sambar","","Jeera-Milagu Rasam"],["","Sprouts (1 small cup)","Ghee Rice","","Bhagara Rice"],["","Tea/Coffee/Milk","French Beans Potato Fry","","Chettinad Chicken Curry (Non-Veg)"],["","Cold Badam Milk without Sugar","Ghobi Masala","","Paneer Green Peas Masala (Veg)"],["","Museli","Fryums","","Snake Gourd Poriyal"],["","Masala Omlet (Non-Veg)","Butter Milk","","Thick Curd"],["","Boiled Red Beans Salad (Veg)","Sorakaya Perugu Chutney, Ghee + Podi","","Water Melon Cut Fruit "],["","","Ghee + Podi","","Gongura Pickle"],["","","Semiya Sabhudhana Kheer","","Milk + Cofee Powder"],["","","","","Mix Veg Raagi Soup"],["Tue\n10, 25","Ghee Podi Dosa (2 Pcs Thin)","Carrot & Cucumber Salad","Aloo Samosa 1 Big Pc","Beetroot & Carrot Salad"],["","Pav Bhaji","Roti","Tomato Sauce, Salt Mirchi","Pulka"],["","Coconut Chutney, Sambar","White Rice","Tea/Coffee/Milk","White Rice"],["","Mint Chutney","Amaranthus Dal","","Dal Makhani"],["","White Bread+Butter+Jam","Beetroot Tomato Rasam","","Kadamba Sambar"],["","Ragi Malt with Salt (1 Medium Glass)","Bisbele Bath","","Mushroom Biryani"],["","Tea/Coffee/Milk","Kakarakaya (Bitter Gourd) Fry","","French Beans Coconut Poriyal"],["","Pomegranate Fresh Juice with Light Sugar","Rajma Masala","","Aloo Mutter Curry"],["","Museli","Kara Boondi","","Thick Curd"],["","Scrambled Egg (Non-Veg)","Dahi Vada","","Banana"],["","Boiled Soya Bean Salad (Veg)","Dosakaya Tomato Chutney","","Tomato Pickle"],["","","Ghee + Podi","","Milk + Cofee Powder"],["","","Kalakand","","Brocolli Soup"],["Wed\n11, 26","Methu Vada (Big size 3pcs, if not 4pcs)","Onions & Lemon Salad","Bandi Style Ragda Chat","Beetroot & Carrot Salad"],["","Mix Veg Suji Bun","Palak Roti","Onion Small pieces","Chapathi"],["","Sambar","White Rice","Ginger Tea/Coffee/Milk","White Rice"],["","Coconut Chutney","Gongura (Sorrel Leaves) Dal","","Tomato Dal"],["","Brown Bread+Peanut Butter+Jam","Rasam","","Drumstick Sambar"],["","Sprouts (1 small cup)","Vegetable Dum Pulav","","Sooji Upma, Coconut Chutney"],["","Tea/Coffee/Milk","Andhra Chicken Fry","","Bottle Guard Tomato Stir Fry"],["","Muskmelon Fresh Juice without Sugar","Kaju Tomato Paneer Fry","","Soya Chunks Masala"],["","Chocos","Red Beans Curry","","Thick Curd"],["","Boiled Egg (Non-Veg)","Trinagle Fryums","","Green Grapes"],["","Boiled Chick Peas Salad (Veg)","Butter Milk","","Lemon Pickle"],["","","Coriander + Tomato Chutney","","Milk + Cofee Powder"],["","","Gulab Jamun","","Tomato Soup with Roasted Bread"],["Thu\n12, 26","Methi Poori (3 Pcs standard size)","Carrot & Cucumber Salad","Boiled Peanuts with Onion/Tomato(12 ) ","Onions & Lemon Salad"],["","Lemon Sevai","Pulka","Boiled Black Chana with Onion/Tomato (26)","Roti"],["","Aloo Mutter Curry (12), Chickpea Masala (26)","White Rice","Tea/Coffee/Milk","White Rice"],["","Coconut Chutney ","Sorrel Leaves Dal (Gongura Pappu)","","Dal Fry"],["","White Bread+Butter+Jam","Drumstick Brinjal Sambar","","Beetroot Tomato Rasam"],["","Sprouts (1 small cup)","Pudina Rice","","Veg Manchuria, Noodles, Tomato Sauce"],["","Tea/Coffee/Milk","Guthi Vankaya Curry","","Chicken 65 / Andhra Fish Fry (NV), Paneer 65 (Veg)"],["","Grapes Fresh Juice without sugar","Ridge Gourd Fry","","Black Channa Curry"],["","Cornflakes","Finger Fryums","","Thick Curd"],["","Masala Onion Omlet (Non-Veg)","Lemon Water + Sabja Seeds","","Banana"],["","Boiled Chickpeas Salad (Veg)","Potlakaya Perugu Chutney","","Gongura Fresh Chutney"],["","","Ghee + Podi","","Milk + Cofee Powder"],["","","Shahi Ki Tukda","","Creamy Mushroom Soup"],["Fri\n13, 27","Uggani + Mirchi Bajji","Beetroot & Cucumber Salad","Aloo Bonda (Medium size 2 pc)","Onions & Lemon Salad"],["","Vegetable Daliya Kitchidi","Chapathi","Coconut Chutney","Lacha Paratha"],["","Coconut Chutney","White Rice","Tea/Coffee/Milk","White Rice"],["","Mango Pickle","Dal Tadka","","Pesara Pappu"],["","Brown Bread+Peanut Butter+Jam","Rasam","","Mix Veg Sambar"],["","Sprouts (1 small cup)","Jeera Rice","","Garlic Chicken Curry (Non-Veg)"],["","Tea/Coffee/Milk","Raw Banana Fry","","Paneer Tikka Masala (Veg)"],["","Sweet Lime + Pineapple Fresh Juice without sugar","Red Rajma Masala","","Tomato Peas Capsicum Masala"],["","Museli","Papad","","Thick Curd"],["","Boiled Egg (Non-Veg)","Butter Milk","","Muskmelon Cut Fruit"],["","Boiled Peanut Sundal (Veg)","Beerakaya Chutney","","Mango Pickle"],["","","Moong Dal Payasam","","Milk + Cofee Powder"],["","","","","Sweet Corn Soup"],["Sat\n14, 28","Mix Veg Utappam - 2 Pcs Std Size","Carrot & Cucumber Salad","Punugulu 8 Pcs Medium Size","Beetroot & Carrot Salad"],["","Vegetable Poha","Pulka","Groundnut/Coconut Chutney","Roti"],["","Peanut Chutney","White Rice","Cold Badam Milk 150 ml","White Rice"],["","Tomato Chutney","Mudda Pappu","","Dal Maharani"],["","White Bread+Butter+Jam","Pachi Pulusu","","Bendakaya (Bhindi) Pulusu"],["","Sprouts (1 small cup)","Pulihora (Tamarind Rice)","","Vegetable Fried Rice, Gobhi Manchuria, Tomato Sauce"],["","Tea/Coffee/Milk","Radish Tomato Stir Fry","","Cabbage Beans Poriyal"],["","Watermelon & Pomegranate Juice with Light Sugar","Chole Curry","","Thick Curd"],["","Chocos","Papad","","Fruit Custard with Custard Apple"],["","Boiled Egg (Non-Veg)","Majiga Pulusu","","Red Chilli Pickle"],["","Sweet Potato Salad (Veg)","Avakaya (Mango Pickle)","","Milk + Cofee Powder"],["","","Ghee + Podi","","Red Lentil Soup"],["","","Carrot Halwa","",""],["","","","",""],["","MESS SERVICE INSTRUCTIONS","","",""],["","1. Thick curd must be served as per the menu, either during lunch or dinner.\n2. Fresh salad must be served every day as per the menu.\n3. The toaster should be kept functional every day.\n4. A weighing machine must be used while serving Chicken and Paneer and it must be functionally available in the mess area everyday.\n5. Chicken should weigh 150 g (and 180 g on Sundays) after cooking, i.e., before serving (excluding bowl weight and gravy). Fish should weight 120 g after cooking (excluding bowl weight)\n6. Paneer/Mushroom should be soft and must weigh 75 g/50 g after cooking, i.e., before serving (excluding bowl weight and gravy).\n7. Roti/Phulka/Chapathi should be prepared using 100% good-quality Atta.\n8. White Rice (Sona Masuri) must be cooked properly every day — neither undercooked nor overcooked.\n9. All wet gravy-based curries should be prepared with 75% vegetables and 25% gravy.\n10. Egg Bhurji should be served as 60 g (after cooking).\n11. The quantity of an item may vary depending on the size served by the caterer.\n12. Fresh juice served in the Special Menu must be atleast 250 ml. Fresh juices, sweet items, and soups included in the Special Menu must be of the highest quality. Please use minimal water/milk while preparing the juices. \n13. Food items like Rava bonda , Vada should be served in 3 standard-sized pieces; if the size is smaller, 4 pieces must be served. (in breakfast)\n14. Ice Cream served during the Sunday lunch should be made primarily with milk and cream. It should not be frozen dessert type.","","",""]]}]"###;
