//! # PhraseForge: A Passphrase Generator
//!
//! PhraseForge generates easy-to-remember passphrases using words from WordNet.
//! The phrases follow the structure: `adjective-noun-verb-adverb`.
//!
//! ## Features
//! - Downloads and extracts WordNet word lists.
//! - Caches word lists locally for offline use.
//! - Generates passphrases using randomly selected words.
//! - Supports re-downloading word lists with a flag.
//! - Allows generating multiple passphrases at once.
//!
//! ## Usage
//! ```sh
//! phraseforge --count 5   # Generate 5 passphrases
//! phraseforge --redownload  # Force re-download of WordNet data
//! ```
//!
//! ## License
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
//! GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License
//! along with this program. If not, see <https://www.gnu.org/licenses/>.
//!
use clap::{Arg, Command as clap_command};
use directories::ProjectDirs;
use inflector::string::pluralize::to_plural;
use rand::prelude::*;
use reqwest::blocking::get;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;

const WORDNET_URL: &str = "https://wordnetcode.princeton.edu/3.0/WNdb-3.0.tar.gz";
const WORDNET_ARCHIVE: &str = "WNdb-3.0.tar.gz";
const HERMIT_DAVES_FREQUENTLY_USED_WORD_LIST_URL: &str =
    "https://raw.githubusercontent.com/hermitdave/FrequencyWords/refs/heads/master/content/2018/en/en_full.txt";
const HERMIT_DAVES_FREQUENTLY_USED_WORD_LIST_ARCHIVE: &str = "en_full.txt";

fn get_data_dir() -> PathBuf {
    let proj_dirs =
        ProjectDirs::from("com", "tynsol", "phraseforge").expect("Failed to get data directory");
    let data_dir = proj_dirs.data_local_dir().to_path_buf();
    fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    data_dir
}

fn download_and_extract_wordnet_dictionary(data_dir: &PathBuf) {
    println!("Downloading WordNet Dictionary...");
    let response = get(WORDNET_URL).expect("Failed to download WordNet");
    let bytes = response.bytes().expect("Failed to read response bytes");

    let archive_path = data_dir.join(WORDNET_ARCHIVE);
    fs::write(&archive_path, &bytes).expect("Failed to save archive");

    println!("Extracting WordNet Dictionary...");
    Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .arg("-C")
        .arg(data_dir)
        .status()
        .expect("Failed to extract WordNet");
    fs::remove_file(&archive_path).expect("Failed to remove archive file");
}

fn download_master_word_list(data_dir: &PathBuf) {
    println!("Downloading Frequently used Word List...");
    let response = get(HERMIT_DAVES_FREQUENTLY_USED_WORD_LIST_URL)
        .expect("Failed to download frequently used word list");

    let archive_path = data_dir.join(HERMIT_DAVES_FREQUENTLY_USED_WORD_LIST_ARCHIVE);
    fs::write(&archive_path, response.bytes().unwrap())
        .expect("Failed to save frequently used word list file");
}

fn generate_word_list(dictionary: &PathBuf, master_word_list: &PathBuf) -> Vec<String> {
    // Step 1: Collect valid first words from the dictionary file
    let dictionary_file = File::open(dictionary).expect("Failed to open dictionary file");
    let dictionary_reader = BufReader::new(dictionary_file);
    let mut dictionary_words: HashSet<String> = HashSet::new();

    for line in dictionary_reader.lines() {
        let line = line.expect("Failed to read line");
        // Extract the first word and check if it starts with an ASCII letter
        let first_word = line.split_whitespace().next().unwrap_or("").to_string();
        if !first_word.is_empty()
            && first_word
                .chars()
                .next()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false)
        {
            dictionary_words.insert(first_word);
        }
    }

    // Step 2: Process the master word list file and include matching lines
    let master_word_list_file =
        File::open(master_word_list).expect("Failed to open word list file");
    let master_word_list_reader = BufReader::new(master_word_list_file);
    let mut word_list = Vec::new();

    for line in master_word_list_reader.lines() {
        let line = line.expect("Failed to read line");
        // Extract the first word from the line in the word list
        let first_word = line.split_whitespace().next().unwrap_or("").to_string();

        // Only include the full line if the first word exists in the dictionary_words set
        if !first_word.is_empty() && dictionary_words.contains(&first_word) {
            word_list.push(line);
        }
    }

    word_list
}

fn save_word_list(words: &[String], file_path: &PathBuf) {
    let mut file = fs::File::create(file_path).expect("Failed to create word list file");
    for word in words {
        writeln!(file, "{}", word).expect("Failed to write word to file");
    }
}

fn word_lists_exist(data_dir: &PathBuf) -> bool {
    let word_files = vec!["adjectives.txt", "nouns.txt", "verbs.txt", "adverbs.txt"];
    word_files.iter().all(|file| data_dir.join(file).exists())
}

fn generate_word_lists(data_dir: &PathBuf) {
    let word_files = vec![
        ("index.adj", "adjectives.txt"),
        ("index.noun", "nouns.txt"),
        ("index.verb", "verbs.txt"),
        ("index.adv", "adverbs.txt"),
    ];

    let dict_dir = data_dir.join("dict");
    let word_list = data_dir.join(HERMIT_DAVES_FREQUENTLY_USED_WORD_LIST_ARCHIVE);
    for (dictionary_file, output_file) in &word_files {
        let dictionary_path = dict_dir.join(dictionary_file);
        let out_path = data_dir.join(output_file);
        let words = generate_word_list(&dictionary_path, &word_list);
        save_word_list(&words, &out_path);
    }
}

fn pick_random_above_frequency(
    word_entries: &[WordEntry],
    min_frequency: &u32,
    rng: &mut ThreadRng,
) -> String {
    let filtered: Vec<&WordEntry> = word_entries
        .iter()
        .filter(|entry| entry.frequency > *min_frequency)
        .collect();

    filtered
        .choose(rng)
        .map(|entry| entry.word.clone())
        .unwrap_or_else(|| "".to_string())
}

fn generate_password(word_lists: &WordLists, min_frequency: &u32) -> String {
    let mut rng = rand::rng();
    let num: u32 = rng.random_range(1..999);

    let adj = if let WordType::Adjective(entries) = &word_lists.adjectives {
        pick_random_above_frequency(entries, min_frequency, &mut rng)
    } else {
        String::new()
    };

    let noun = if let WordType::Noun(entries) = &word_lists.nouns {
        let n = pick_random_above_frequency(entries, min_frequency, &mut rng);
        if num > 1 && !n.is_empty() {
            to_plural(&n)
        } else {
            n
        }
    } else {
        String::new()
    };

    let verb = if let WordType::Verb(entries) = &word_lists.verbs {
        pick_random_above_frequency(entries, min_frequency, &mut rng)
    } else {
        String::new()
    };

    let adv = if let WordType::Adverb(entries) = &word_lists.adverbs {
        pick_random_above_frequency(entries, min_frequency, &mut rng)
    } else {
        String::new()
    };

    format!("{}-{}-{}-{}-{}", num, adj, noun, verb, adv)
}

#[derive(Debug)]
struct WordEntry {
    word: String,
    frequency: u32,
}

#[derive(Debug)]
enum WordType {
    Adjective(Vec<WordEntry>),
    Noun(Vec<WordEntry>),
    Verb(Vec<WordEntry>),
    Adverb(Vec<WordEntry>),
}

#[derive(Debug)]
struct WordLists {
    adjectives: WordType,
    nouns: WordType,
    verbs: WordType,
    adverbs: WordType,
}

fn load_word_list(word_list: &PathBuf) -> Vec<WordEntry> {
    let file = File::open(word_list).expect("Failed to open word list file.");
    let reader = BufReader::new(file);

    reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let mut parts = line.split_whitespace();
            let word = parts.next()?.to_string();
            let freq_str = parts.next()?;
            let frequency = freq_str.parse::<u32>().ok()?;
            Some(WordEntry { word, frequency })
        })
        .collect()
}

fn load_all_word_lists(base_path: &PathBuf) -> WordLists {
    let adjectives = load_word_list(&base_path.join("adjectives.txt"));
    let nouns = load_word_list(&base_path.join("nouns.txt"));
    let verbs = load_word_list(&base_path.join("verbs.txt"));
    let adverbs = load_word_list(&base_path.join("adverbs.txt"));
    WordLists {
        adjectives: WordType::Adjective(adjectives),
        nouns: WordType::Noun(nouns),
        verbs: WordType::Verb(verbs),
        adverbs: WordType::Adverb(adverbs),
    }
}

fn load_or_generate_word_lists(data_dir: &PathBuf, force_download: bool) -> WordLists {
    if !word_lists_exist(data_dir) || force_download {
        download_and_extract_wordnet_dictionary(data_dir);
        download_master_word_list(data_dir);
        generate_word_lists(data_dir);
    }

    let wordlists = load_all_word_lists(&data_dir);
    wordlists
}

fn parse_arguments() -> clap::ArgMatches {
    clap_command::new("PhraseForge")
        .version("0.1.0")
        .author("Chris Solomon <chris.m.solomon@gmail.com>")
        .about("Generates memorable passphrases using WordNet word lists")
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .help("Number of passphrases to generate")
                .value_parser(clap::value_parser!(usize))
                .default_value("1"),
        )
        .arg(
            Arg::new("min-frequency")
                .short('f')
                .long("min-frequency")
                .help("Minimum word frequency to include")
                .value_parser(clap::value_parser!(u32))
                .default_value("10000"),
        )
        .arg(
            Arg::new("redownload")
                .short('r')
                .long("redownload")
                .help("Force re-download of WordNet data")
                .num_args(0),
        )
        .get_matches()
}

fn main() {
    env_logger::init(); // Reads RUST_LOG from the environment

    let matches = parse_arguments();
    log::debug!("Command line arguments: {:?}", matches);

    let num_passwords = *matches.get_one::<usize>("count").unwrap();
    let force_download = matches.get_flag("redownload");
    let min_frequency: u32 = *matches.get_one::<u32>("min-frequency").unwrap();

    let data_dir = get_data_dir();
    let word_lists = load_or_generate_word_lists(&data_dir, force_download);

    for _ in 0..num_passwords {
        println!("{}", generate_password(&word_lists, &min_frequency));
    }
}
