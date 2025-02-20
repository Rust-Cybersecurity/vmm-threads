use kvm_ioctls::{Kvm, VcpuFd, VmFd};
use kvm_bindings::{kvm_userspace_memory_region, KVM_MEM_LOG_DIRTY_PAGES};
use std::ptr;

// Struct to encapsulate VMM state
struct Vmm {
    kvm: Kvm,
    vm: VmFd,
    vcpu: VcpuFd,
    guest_mem: *mut libc::c_void,
    mem_size: usize,
}

impl Vmm {
    // Initialize the VMM
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Open KVM
        let kvm = Kvm::new()?;
        println!("KVM API version: {}", kvm.get_api_version());

        // Create a VM
        let vm = kvm.create_vm()?;

        // Allocate guest memory (e.g., 16 MiB for simplicity)
        let mem_size = 16 * 1024 * 1024; // 16 MiB
        let guest_addr = 0x1000; // Starting guest physical address
        let guest_mem = unsafe {
            libc::mmap(
                ptr::null_mut(),
                mem_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_PRIVATE,
                -1,
                0,
            )
        };
        if guest_mem == libc::MAP_FAILED {
            return Err("Failed to allocate guest memory".into());
        }

        // Register memory with KVM
        let mem_region = kvm_userspace_memory_region {
            slot: 0,
            flags: KVM_MEM_LOG_DIRTY_PAGES,
            guest_phys_addr: guest_addr as u64,
            memory_size: mem_size as u64,
            userspace_addr: guest_mem as u64,
        };
        unsafe {
            vm.set_user_memory_region(mem_region)?;
        }

        // Create a single vCPU
        let vcpu = vm.create_vcpu(0)?;

        Ok(Vmm {
            kvm,
            vm,
            vcpu,
            guest_mem,
            mem_size,
        })
    }

    // Configure the vCPU and load a minimal guest program
    fn setup_vcpu(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Set up segment registers (sregs)
        let mut sregs = self.vcpu.get_sregs()?;
        sregs.cs.base = 0;
        sregs.cs.selector = 0;
        self.vcpu.set_sregs(&sregs)?;

        // Set up general-purpose registers (regs), including rip
        let mut regs = self.vcpu.get_regs()?;
        regs.rip = 0x1000; // Point to start of guest memory
        self.vcpu.set_regs(&regs)?;

        // Load a tiny program: infinite loop (0xeb 0xfe = jmp $)
        let guest_code: &[u8] = &[0xeb, 0xfe];
        unsafe {
            std::ptr::copy_nonoverlapping(
                guest_code.as_ptr(),
                self.guest_mem as *mut u8,
                guest_code.len(),
            );
        }

        Ok(())
    }

    // Run the VMM thread's main loop
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("VMM thread running...");
        loop {
            match self.vcpu.run() {
                Ok(run) => match run {
                    kvm_ioctls::VcpuExit::Hlt => {
                        println!("Guest halted");
                        break;
                    }
                    kvm_ioctls::VcpuExit::IoIn(addr, _data) => {
                        println!("IO in at port 0x{:x}", addr);
                        // In Firecracker, this might trigger device emulation
                    }
                    kvm_ioctls::VcpuExit::IoOut(addr, data) => {
                        println!("IO out at port 0x{:x} with data {:?}", addr, data);
                        // Minimal emulation could go here
                    }
                    unexpected => {
                        println!("Unexpected exit: {:?}", unexpected);
                        break;
                    }
                },
                Err(e) => {
                    println!("vCPU run failed: {:?}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    // Clean up resources
    fn cleanup(&mut self) {
        unsafe {
            libc::munmap(self.guest_mem, self.mem_size);
        }
    }
}

impl Drop for Vmm {
    fn drop(&mut self) {
        self.cleanup();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vmm = Vmm::new()?;
//    vmm.setup_vcpu()?;
//    vmm.run()?;
    Ok(())
}

// Error handling boilerplate
//impl From<std::io::Error> for Box<dyn std::error::Error> {
//    fn from(err: std::io::Error) -> Self {
//        Box::new(err)
//    }
//}