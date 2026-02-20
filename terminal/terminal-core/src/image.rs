use std::collections::HashMap;

const AUTO_IMAGE_ID_START: u32 = 0x8000_0000;
const PLACEMENT_ID_START: u32 = 0x4000_0000;

#[derive(Debug, Clone)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl ImageData {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba,
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> (u8, u8, u8, u8) {
        if x >= self.width || y >= self.height {
            return (0, 0, 0, 0);
        }
        let idx = ((y * self.width + x) * 4) as usize;
        if idx + 3 < self.rgba.len() {
            (
                self.rgba[idx],
                self.rgba[idx + 1],
                self.rgba[idx + 2],
                self.rgba[idx + 3],
            )
        } else {
            (0, 0, 0, 0)
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlacedImage {
    pub id: u32,
    pub image_id: u32,
    pub col: usize,
    pub row: usize,
    pub width_cells: usize,
    pub height_cells: usize,
    pub x_offset: u32,
    pub y_offset: u32,
    pub source_x: u32,
    pub source_y: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub z_index: i32,
}

#[derive(Debug, Clone)]
pub struct ImageStore {
    images: HashMap<u32, ImageData>,
    placements: Vec<PlacedImage>,
    next_auto_image_id: u32,
    next_placement_id: u32,
}

impl ImageStore {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
            placements: Vec::new(),
            next_auto_image_id: AUTO_IMAGE_ID_START,
            next_placement_id: PLACEMENT_ID_START,
        }
    }

    fn alloc_image_id(&mut self) -> u32 {
        loop {
            let id = self.next_auto_image_id;
            self.next_auto_image_id = self.next_auto_image_id.wrapping_add(1);
            if id != 0 && !self.images.contains_key(&id) {
                return id;
            }
        }
    }

    fn alloc_placement_id(&mut self) -> u32 {
        loop {
            let id = self.next_placement_id;
            self.next_placement_id = self.next_placement_id.wrapping_add(1);
            if id != 0 && !self.placements.iter().any(|p| p.id == id) {
                return id;
            }
        }
    }

    pub fn store_image(&mut self, data: ImageData) -> u32 {
        let id = self.alloc_image_id();
        self.images.insert(id, data);
        id
    }

    pub fn store_image_with_id(&mut self, id: u32, data: ImageData) -> u32 {
        if id == 0 {
            return self.store_image(data);
        }
        self.images.insert(id, data);
        id
    }

    pub fn get_image(&self, id: u32) -> Option<&ImageData> {
        self.images.get(&id)
    }

    pub fn remove_image(&mut self, id: u32) {
        self.images.remove(&id);
        self.placements.retain(|p| p.image_id != id);
    }

    pub fn place_image(
        &mut self,
        image_id: u32,
        col: usize,
        row: usize,
        width_cells: usize,
        height_cells: usize,
    ) -> u32 {
        let (source_width, source_height) = if let Some(img) = self.images.get(&image_id) {
            (img.width, img.height)
        } else {
            return 0;
        };

        let placement_id = self.alloc_placement_id();
        self.placements.push(PlacedImage {
            id: placement_id,
            image_id,
            col,
            row,
            width_cells,
            height_cells,
            x_offset: 0,
            y_offset: 0,
            source_x: 0,
            source_y: 0,
            source_width,
            source_height,
            z_index: 0,
        });
        placement_id
    }

    pub fn place_image_detailed(&mut self, mut placement: PlacedImage) {
        if placement.id == 0 {
            placement.id = self.alloc_placement_id();
        }
        self.placements.push(placement);
    }

    pub fn remove_placement(&mut self, placement_id: u32) {
        self.placements.retain(|p| p.id != placement_id);
    }

    pub fn remove_placements_at_cell(&mut self, col: usize, row: usize) {
        self.placements.retain(|p| !(p.col == col && p.row == row));
    }

    pub fn remove_placements_in_column(&mut self, col: usize) {
        self.placements.retain(|p| p.col != col);
    }

    pub fn remove_placements_in_row(&mut self, row: usize) {
        self.placements.retain(|p| p.row != row);
    }

    pub fn remove_placements_with_z_index(&mut self, z_index: i32) {
        self.placements.retain(|p| p.z_index != z_index);
    }

    pub fn placements(&self) -> &[PlacedImage] {
        &self.placements
    }

    pub fn placements_in_region(&self, start_row: usize, end_row: usize) -> Vec<&PlacedImage> {
        self.placements
            .iter()
            .filter(|p| {
                let height = p.height_cells.max(1);
                let p_end = p.row.saturating_add(height);
                p.row < end_row && p_end > start_row
            })
            .collect()
    }

    pub fn scroll_up(&mut self, top: usize, bottom: usize, n: usize) {
        if n == 0 {
            return;
        }
        self.placements.retain_mut(|p| {
            if p.row < top || p.row > bottom {
                return true;
            }
            if p.row < top.saturating_add(n) {
                return false;
            }
            p.row = p.row.saturating_sub(n);
            true
        });
    }

    pub fn scroll_down(&mut self, top: usize, bottom: usize, n: usize) {
        if n == 0 {
            return;
        }
        self.placements.retain_mut(|p| {
            if p.row < top || p.row > bottom {
                return true;
            }
            let new_row = p.row.saturating_add(n);
            if new_row > bottom {
                return false;
            }
            p.row = new_row;
            true
        });
    }

    pub fn clear(&mut self) {
        self.images.clear();
        self.placements.clear();
    }

    pub fn clear_placements(&mut self) {
        self.placements.clear();
    }

    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    pub fn placement_count(&self) -> usize {
        self.placements.len()
    }
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_data_pixel() {
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 128];
        let img = ImageData::new(2, 1, rgba);
        assert_eq!(img.pixel(0, 0), (255, 0, 0, 255));
        assert_eq!(img.pixel(1, 0), (0, 255, 0, 128));
        assert_eq!(img.pixel(2, 0), (0, 0, 0, 0));
    }

    #[test]
    fn test_image_store_basic() {
        let mut store = ImageStore::new();
        let data = ImageData::new(10, 10, vec![0; 400]);
        let id = store.store_image(data);
        assert!(id != 0);
        assert!(store.get_image(id).is_some());
        assert_eq!(store.image_count(), 1);
    }

    #[test]
    fn test_image_store_with_id() {
        let mut store = ImageStore::new();
        let data = ImageData::new(2, 2, vec![0; 16]);
        let id = store.store_image_with_id(123, data);
        assert_eq!(id, 123);
        assert!(store.get_image(123).is_some());
    }

    #[test]
    fn test_image_store_placement() {
        let mut store = ImageStore::new();
        let data = ImageData::new(10, 10, vec![0; 400]);
        let img_id = store.store_image(data);
        let p_id = store.place_image(img_id, 0, 0, 5, 5);
        assert!(p_id != 0);
        assert_eq!(store.placement_count(), 1);
    }

    #[test]
    fn test_image_store_remove() {
        let mut store = ImageStore::new();
        let data = ImageData::new(10, 10, vec![0; 400]);
        let img_id = store.store_image(data);
        store.place_image(img_id, 0, 0, 5, 5);
        store.remove_image(img_id);
        assert_eq!(store.image_count(), 0);
        assert_eq!(store.placement_count(), 0);
    }

    #[test]
    fn test_placements_in_region() {
        let mut store = ImageStore::new();
        let data = ImageData::new(10, 10, vec![0; 400]);
        let img_id = store.store_image(data);
        store.place_image(img_id, 0, 5, 3, 3);
        store.place_image(img_id, 0, 20, 3, 3);

        let visible = store.placements_in_region(0, 10);
        assert_eq!(visible.len(), 1);

        let visible = store.placements_in_region(0, 25);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn test_scroll_up_down() {
        let mut store = ImageStore::new();
        let data = ImageData::new(10, 10, vec![0; 400]);
        let img_id = store.store_image(data);
        store.place_image(img_id, 0, 5, 3, 3);
        store.scroll_up(0, 10, 2);
        assert_eq!(store.placements()[0].row, 3);
        store.scroll_down(0, 10, 4);
        assert_eq!(store.placements()[0].row, 7);
    }
}
