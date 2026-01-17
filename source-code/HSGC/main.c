#include "hsgc.h"
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <assert.h>

// Simple linked list for all allocated objects
struct HSGC_Object {
    struct HSGC_Object* next;
    int marked; // 0 = unmarked, 1 = marked
    // User data follows here (flexible array)
    char data[]; // Placeholder for allocated data
};

// Global list of all objects
static HSGC_Object* all_objects = NULL;

// List of roots (pointers to pointers)
typedef struct Root {
    void** ptr;
    struct Root* next;
} Root;

static Root* roots = NULL;

// Helper to add object to list
static void add_object(HSGC_Object* obj) {
    obj->next = all_objects;
    all_objects = obj;
}

// Init GC
void hsgc_init(void) {
    // Nothing special for now
}

// Alloc
void* hsgc_alloc(size_t size) {
    HSGC_Object* obj = malloc(sizeof(HSGC_Object) + size);
    if (!obj) {
        fprintf(stderr, "GC alloc failed\n");
        exit(1);
    }
    obj->marked = 0;
    add_object(obj);
    return obj->data;
}

// Mark phase: mark from roots
static void mark_all(void) {
    // Mark all objects reachable from roots
    for (Root* r = roots; r; r = r->next) {
        if (*r->ptr) {
            HSGC_Object* obj = (HSGC_Object*)((char*)(*r->ptr) - offsetof(HSGC_Object, data));
            if (!obj->marked) {
                obj->marked = 1;
                // For simplicity, assume no pointers inside objects; extend for real use
            }
        }
    }
}

// Sweep phase: free unmarked
static void sweep(void) {
    HSGC_Object** ptr = &all_objects;
    while (*ptr) {
        HSGC_Object* obj = *ptr;
        if (obj->marked) {
            obj->marked = 0; // Reset for next collection
            ptr = &obj->next;
        } else {
            *ptr = obj->next;
            free(obj);
        }
    }
}

// Collect
void hsgc_collect(void) {
    mark_all();
    sweep();
}

// Mark a specific object (if needed)
void hsgc_mark(HSGC_Object* obj) {
    if (obj && !obj->marked) {
        obj->marked = 1;
        // Recurse if there are pointers inside (not implemented here)
    }
}

// Register root
void hsgc_register_root(void** root) {
    Root* new_root = malloc(sizeof(Root));
    if (!new_root) {
        fprintf(stderr, "Root alloc failed\n");
        exit(1);
    }
    new_root->ptr = root;
    new_root->next = roots;
    roots = new_root;
}

// Unregister root
void hsgc_unregister_root(void** root) {
    Root** ptr = &roots;
    while (*ptr) {
        Root* r = *ptr;
        if (r->ptr == root) {
            *ptr = r->next;
            free(r);
            return;
        }
        ptr = &r->next;
    }
}

// Fini
void hsgc_fini(void) {
    hsgc_collect(); // Final collection
    // Free remaining objects (should be none if all freed)
    while (all_objects) {
        HSGC_Object* obj = all_objects;
        all_objects = obj->next;
        free(obj);
    }
    // Free roots
    while (roots) {
        Root* r = roots;
        roots = r->next;
        free(r);
    }
}
