/* Pi-hole: A black hole for Internet advertisements
*  (c) 2018 Pi-hole, LLC (https://pi-hole.net)
*  Network-wide ad blocking via your own hardware.
*
*  API
*  List structure and operations for DNS endpoints
*
*  This file is copyright under the latest version of the EUPL.
*  Please see LICENSE file for your rights under this license. */

use std::io::prelude::*;
use std::io::{self, BufReader, BufWriter};

use util;
use dns::common::{is_valid_domain, is_valid_regex};
use config::{Config, PiholeFile};

pub enum List {
    White, Black, Regex
}

impl List {
    /// Get the associated `PiholeFile`
    fn file(&self) -> PiholeFile {
        match *self {
            List::White => PiholeFile::Whitelist,
            List::Black => PiholeFile::Blacklist,
            List::Regex => PiholeFile::Regexlist
        }
    }

    /// Check if the list accepts the domain as valid
    fn accepts(&self, domain: &str) -> bool {
        match *self {
            List::Regex => is_valid_regex(domain),
            _ => is_valid_domain(domain)
        }
    }

    /// Read in the domains from the list
    pub fn get(&self, config: &Config) -> Result<Vec<String>, util::Error> {
        let file = match config.read_file(self.file()) {
            Ok(f) => f,
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    // If the file is not found, then the list is empty
                    return Ok(Vec::new());
                } else {
                    return Err(e.into());
                }
            }
        };

        Ok(
            BufReader::new(file)
                .lines()
                .filter_map(|line| line.ok())
                .filter(|line| line.len() != 0)
                .collect()
        )
    }

    /// Add a domain to the list
    pub fn add(&self, domain: &str, config: &Config) -> Result<(), util::Error> {
        // Check if it's a valid domain before doing anything
        if !self.accepts(domain) {
            return Err(util::Error::InvalidDomain);
        }

        // Check if the domain is already in the list
        if self.get(config)?.contains(&domain.to_owned()) {
            return Err(util::Error::AlreadyExists);
        }

        // Open the list file in append mode (and create it if it doesn't exist)
        let mut file = config.write_file(self.file(), true)?;

        // Add the domain to the list
        writeln!(file, "{}", domain)?;

        Ok(())
    }

    /// Try to remove a domain from the list, but it is not an error if the domain does not exist
    pub fn try_remove(&self, domain: &str, config: &Config) -> Result<(), util::Error> {
        match self.remove(domain, config) {
            // Pass through successful results
            Ok(ok) => Ok(ok),
            Err(e) => {
                // Ignore NotFound errors
                if e == util::Error::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Remove a domain from the list
    pub fn remove(&self, domain: &str, config: &Config) -> Result<(), util::Error> {
        // Check if it's a valid domain before doing anything
        if !self.accepts(domain) {
            return Err(util::Error::InvalidDomain);
        }

        // Check if the domain is not in the list
        let domains = self.get(config)?;
        if !domains.contains(&domain.to_owned()) {
            return Err(util::Error::NotFound);
        }

        // Open the list file (and create it if it doesn't exist). This will truncate the list so
        // we can add all the domains except the one we are deleting
        let file = config.write_file(self.file(), false)?;
        let mut writer = BufWriter::new(file);

        // Write all domains except the one we're deleting
        for domain in domains.into_iter().filter(|item| item != domain) {
            writeln!(writer, "{}", domain)?;
        }

        Ok(())
    }
}
