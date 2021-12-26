use bitfield::bitfield;

bitfield! {
    /// See Figure 5-25. (2-Mbyte PDE—Long Mode)
    pub struct LegacyPDE(u64);
    pub get_valid, set_valid: 0, 0;                             // [0]
    pub get_write, set_write: 1, 1;                             // [1]
    pub get_user, set_user: 2, 2;                               // [2]
    pub get_write_through, set_write_through: 3, 3;             // [3]
    pub get_cache_disable, set_cache_disable: 4, 4;             // [4]
    pub get_accessed, set_accessed: 5, 5;                       // [5]
    pub get_dirty, set_dirty: 6, 6;                             // [6]
    pub get_large_page, set_large_page: 7, 7;                   // [7]
    pub get_global, set_global: 8, 8;                           // [8]
    pub get_avl, set_avl: 11, 9;                                // [9-11]
    pub get_pat, set_pat: 12, 12;                               // [12]
    // reserved                                                 // [13-20]
    pub get_page_frame_number, set_page_frame_number: 51, 21;   // [21-51]
    // reserved                                                 // [52-62]
    pub get_no_execute, set_no_execute: 63, 63;                 // [63]
}

bitfield! {
    /// See Figure 5-23. (2-Mbyte PML4E—Long Mode)
    pub struct LegacyPML4E(u64);
    pub get_valid, set_valid: 0, 0;                             // [0]
    pub get_write, set_write: 1, 1;                             // [1]
    pub get_user, set_user: 2, 2;                               // [2]
    pub get_write_through, set_write_through: 3, 3;             // [3]
    pub get_cache_disable, set_cache_disable: 4, 4;             // [4]
    pub get_accessed, set_accessed: 5, 5;                       // [5]
    // reserved                                                 // [6-8]
    pub get_avl, set_avl: 11, 9;                                // [9-11]
    pub get_page_frame_number, set_page_frame_number: 51, 12;   // [12-51]
    // reserved                                                 // [52-62]
    pub get_no_execute, set_no_execute: 63, 63;                 // [63]
}
