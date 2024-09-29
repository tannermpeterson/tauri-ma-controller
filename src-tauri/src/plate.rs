use anyhow::{bail, Context, Result};

// TODO make everything work for larger plates

pub struct Plate {
    num_rows: usize,
    num_cols: usize,
}

impl Plate {
    fn new(num_rows: usize, num_cols: usize) -> Self {
        Self { num_rows, num_cols }
    }

    pub fn new_24() -> Self {
        Self::new(4, 6)
    }
    pub fn new_96() -> Self {
        Self::new(8, 12)
    }
    pub fn new_384() -> Self {
        Self::new(16, 24)
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn num_cols(&self) -> usize {
        self.num_cols
    }

    pub fn num_wells(&self) -> usize {
        self.num_rows * self.num_cols
    }

    /// Get 0-based index of well name
    pub fn name_to_idx(&self, well_name: String) -> Result<usize> {
        let (row_idx, col_idx) = Self::name_to_row_col(well_name)?;
        self.row_col_to_idx(row_idx, col_idx)
    }

    /// Get well name from 0-based index
    pub fn idx_to_name(&self, well_idx: usize, pad_zeros: bool) -> Result<String> {
        let (row_idx, col_idx) = self.idx_to_row_col(well_idx)?;
        self.row_col_to_name(row_idx, col_idx, pad_zeros)
    }

    pub fn name_to_row_col(well_name: String) -> Result<(usize, usize)> {
        if !well_name.is_ascii() {
            bail!("Invalid well name: {}", well_name);
        }

        let row_name: char = well_name.chars().take(1).collect::<Vec<char>>()[0];
        if row_name < 'A' || 'P' < row_name {
            bail!("Invalid row: {}", row_name);
        }
        let row_idx = row_name as usize - 'A' as usize;

        let col_name = &well_name[1..];
        let col: usize = col_name
            .parse()
            .with_context(|| format!("Invalid col: {}", col_name))?;
        if col < 1 || col > 24 {
            bail!("Invalid col: {}", col_name);
        }
        let col_idx = col - 1;

        Ok((row_idx, col_idx))
    }

    pub fn row_col_to_name(
        &self,
        row_idx: usize,
        col_idx: usize,
        pad_zeros: bool,
    ) -> Result<String> {
        let row_str = Self::row_idx_to_name(row_idx)?;
        let col_str = self.get_formatted_col_string(col_idx, pad_zeros)?;
        Ok(format!("{row_str}{col_str}"))
    }

    pub fn idx_to_row_col(&self, well_idx: usize) -> Result<(usize, usize)> {
        self.validate_well_idx(well_idx)?;
        let row = well_idx % self.num_rows;
        let col = well_idx / self.num_rows;
        Ok((row, col))
    }

    pub fn row_col_to_idx(&self, row_idx: usize, col_idx: usize) -> Result<usize> {
        self.validate_row_idx(row_idx)?;
        self.validate_col_idx(col_idx)?;
        Ok(row_idx + (col_idx * self.num_rows))
    }

    // pub fn row_names(&self)

    // pub fn col_names(&self, pad_zeros: bool)

    pub fn row_idx_to_name(row_idx: usize) -> Result<char> {
        Self::validate_row_idx_max(row_idx)?;
        Ok(char::from_u32('A' as u32 + row_idx as u32).expect("row_idx validation failed"))
    }

    /// Validate based on max plate size
    fn validate_well_idx_max(well_idx: usize) -> Result<()> {
        if well_idx >= 384 {
            // TODO make a const for this upper bound
            bail!("Invalid well idx: {}", well_idx);
        }
        Ok(())
    }

    /// Validate based on max plate size
    fn validate_row_idx_max(row_idx: usize) -> Result<()> {
        if row_idx >= 16 {
            // TODO make a const for this upper bound
            bail!("Invalid row idx: {}", row_idx);
        }
        Ok(())
    }

    /// Validate based on max plate size
    fn validate_col_idx_max(col_idx: usize) -> Result<()> {
        if col_idx >= 24 {
            // TODO make a const for this upper bound
            bail!("Invalid col idx: {}", col_idx);
        }
        Ok(())
    }

    /// Validate based on this plate's size
    fn validate_well_idx(&self, well_idx: usize) -> Result<()> {
        if well_idx >= self.num_wells() {
            bail!("Invalid well idx: {}", well_idx);
        }
        Ok(())
    }

    /// Validate based on this plate's size
    fn validate_row_idx(&self, row_idx: usize) -> Result<()> {
        if row_idx >= self.num_rows {
            bail!("Invalid row idx: {}", row_idx);
        }
        Ok(())
    }

    /// Validate based on this plate' size
    fn validate_col_idx(&self, col_idx: usize) -> Result<()> {
        if col_idx >= self.num_cols {
            bail!("Invalid col idx: {}", col_idx);
        }
        Ok(())
    }

    fn get_formatted_col_string(&self, col_idx: usize, pad_zeros: bool) -> Result<String> {
        self.validate_col_idx(col_idx)?;
        let mut col_name = (col_idx + 1).to_string();
        if pad_zeros {
            let max_num_digits = self.num_cols.to_string().len();
            col_name = format!("{col_name:0>max_num_digits$}");
        }
        Ok(col_name)
    }
}

// fn is_well_name(name: String) -> bool:
//     if match := re.match(r"^[A-P](\d{1,2})$", name):
//         return 1 <= usize(match.groups()[0]) <= 24
//     return False
//
//
// fn get_smallest_plate(wells: list[String]) -> usize
//     // TODO try splitting all well names into row col idxs, then take the greater well size from
//     // the max row idx and max col idx
//     smallest_plate_size = 0
//
//     for w in wells:
//         if not is_well_name(w):
//             raise ValueError(f"Invalid Well: {w}")
//
//         row, col = name_to_row_col(w)
//         smallest_size_containing_well = next(
//             n_wells for n_wells, dims in Plate.plate_dimensions.items() if row < dims[0] and col < dims[1]
//         )
//         smallest_plate_size = max(smallest_plate_size, smallest_size_containing_well)
//
//     return smallest_plate_size

#[cfg(test)]
mod tests {
    use super::*;

    mod name_to_idx {
        use super::*;
        // TODO
    }

    mod idx_to_name {
        use super::*;
        // TODO
    }

    mod row_col_to_name {
        use super::*;

        #[test]
        fn valid_idxs() {
            let p24 = Plate::new_24();
            let res = p24.row_col_to_name(0, 0, false).expect("should be valid");
            assert_eq!(res, "A1");
            let res = p24.row_col_to_name(3, 5, false).expect("should be valid");
            assert_eq!(res, "D6");
            let p96 = Plate::new_96();
            let res = p96.row_col_to_name(0, 0, false).expect("should be valid");
            assert_eq!(res, "A1");
            let res = p96.row_col_to_name(7, 11, false).expect("should be valid");
            assert_eq!(res, "H12");
            let p384 = Plate::new_384();
            let res = p384.row_col_to_name(0, 0, false).expect("should be valid");
            assert_eq!(res, "A1");
            let res = p384
                .row_col_to_name(15, 23, false)
                .expect("should be valid");
            assert_eq!(res, "P24");
        }

        #[test]
        fn valid_idxs_with_padding() {
            let p24 = Plate::new_24();
            let res = p24.row_col_to_name(0, 0, true).expect("should be valid");
            assert_eq!(res, "A1");
            let res = p24.row_col_to_name(3, 5, true).expect("should be valid");
            assert_eq!(res, "D6");
            let p96 = Plate::new_96();
            let res = p96.row_col_to_name(0, 0, true).expect("should be valid");
            assert_eq!(res, "A01");
            let res = p96.row_col_to_name(7, 11, true).expect("should be valid");
            assert_eq!(res, "H12");
            let p384 = Plate::new_384();
            let res = p384.row_col_to_name(0, 0, true).expect("should be valid");
            assert_eq!(res, "A01");
            let res = p384.row_col_to_name(15, 23, true).expect("should be valid");
            assert_eq!(res, "P24");
        }

        // TODO invalid idxs
    }

    mod name_to_row_col {
        use super::*;

        #[test]
        fn parse_valid_well_names() {
            let res = Plate::name_to_row_col("A1".to_string()).expect("should parse");
            assert_eq!(res, (0, 0));
            let res = Plate::name_to_row_col("P24".to_string()).expect("should parse");
            assert_eq!(res, (15, 23));
            let res = Plate::name_to_row_col("D0010".to_string()).expect("should parse");
            assert_eq!(res, (3, 9));
        }

        #[test]
        fn parse_invalid_ascii() {
            // invalid row
            let res = Plate::name_to_row_col("@1".to_string()).expect_err("should err");
            assert!(res.to_string().contains("Invalid row: @"));
            let res = Plate::name_to_row_col("Q1".to_string()).expect_err("should err");
            assert!(res.to_string().contains("Invalid row: Q"));
            // invalid col
            let res = Plate::name_to_row_col("A0".to_string()).expect_err("should err");
            assert!(res.to_string().contains("Invalid col: 0"));
            let res = Plate::name_to_row_col("A25".to_string()).expect_err("should err");
            assert!(res.to_string().contains("Invalid col: 25"));
            let res = Plate::name_to_row_col("A-1".to_string()).expect_err("should err");
            assert!(res.to_string().contains("Invalid col: -1"));
            let res = Plate::name_to_row_col("AA".to_string()).expect_err("should err");
            assert!(res.to_string().contains("Invalid col: A"));
        }

        // TODO invalid unicode
    }

    mod row_idx_to_name {
        use super::*;

        #[test]
        fn parse() {
            let res = Plate::row_idx_to_name(0).expect("should be valid row");
            assert_eq!(res, 'A');
            let res = Plate::row_idx_to_name(15).expect("should be valid row");
            assert_eq!(res, 'P');
            let res = Plate::row_idx_to_name(16).expect_err("should be invalid row");
            assert!(res.to_string().contains("Invalid row idx: 16"));
        }
    }

    mod idx_to_row_col {
        use super::*;
        // TODO
    }

    mod row_col_to_idx {
        use super::*;
        // TODO
    }
}
